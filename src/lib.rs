use failure::{bail, err_msg, format_err, Fallible};
use log::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
    time::Duration,
};

trait EasyExec {
    fn easy_exec(&mut self) -> Fallible<()>;
}

impl EasyExec for Command {
    fn easy_exec(&mut self) -> Fallible<()> {
        let output = self
            .output()
            .map_err(|_| format_err!("Command not found: {:?}", self))?;
        if !output.status.success() {
            let out =
                String::from_utf8(output.stdout).map_err(|_| err_msg("Failed to parse output"))?;
            let err = String::from_utf8(output.stderr.clone())
                .map_err(|_| err_msg("Failed to parse error output"))?;
            error!("Exit command: {:?}", self);
            error!("Error code: {}", output.status.code().unwrap());
            error!("{}", out);
            error!("{}", err);
        }
        Ok(())
    }
}

/// Download and Build latest version of flatc
pub fn build_flatc() -> Fallible<PathBuf> {
    let work_dir = dirs::cache_dir()
        .expect("Cannot get global cache directory")
        .join("flatc-gen");
    fs::create_dir_all(&work_dir).expect("Failed to create cache directory");
    info!("Use global cache dir: {}", work_dir.display());

    // inter-process exclusion (parallel cmake will cause problems)
    let lock_file = work_dir.join("flatc-gen.lock");
    fs::File::create(&lock_file).expect("Cannot create lock file");
    let mut count = 0;
    let _lock = loop {
        match file_lock::FileLock::lock(lock_file.to_str().unwrap(), true, true) {
            Ok(lock) => break lock,
            Err(err) => {
                count += 1;
                warn!("Waiting lock of {}, {:?}", lock_file.display(), err);
            }
        };
        // Try 30s to get lock
        if count > 30 {
            panic!("Cannot get lock of {} in 30s", lock_file.display());
        }
        sleep(Duration::from_secs(1));
    };

    // FIXME use release version instead of HEAD
    let fbs_repo = work_dir.join("flatbuffers");
    if !fbs_repo.exists() {
        Command::new("git")
            .args(&["clone", "http://github.com/google/flatbuffers"])
            .current_dir(&work_dir)
            .easy_exec()?;
    }

    // Build flatbuffers
    Command::new("cmake")
        .args(&["-Bbuild", "-H."])
        .current_dir(&fbs_repo)
        .easy_exec()?;
    Command::new("cmake")
        .args(&["--build", "build", "--target", "flatc"])
        .current_dir(&fbs_repo)
        .easy_exec()?;

    Ok(fbs_repo.join("build/flatc"))
}

/// Generate Rust code from FlatBuffer definitions
pub fn flatc_gen(path: impl AsRef<Path>, out_dir: impl AsRef<Path>) -> Fallible<()> {
    let path = path.as_ref();
    if !path.exists() {
        bail!("Flatbuffer file {} does not exist.", path.display());
    }
    let flatc = build_flatc()?;
    Command::new(flatc)
        .arg("--rust")
        .arg("-o")
        .arg(out_dir.as_ref())
        .arg("-b")
        .arg(path)
        .easy_exec()
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_flatc() {
        super::build_flatc().unwrap();
    }

    #[test]
    fn flatc_gen() {
        // Be sure that this test runs at the top of this repo
        super::flatc_gen("fbs/addressbook.fbs", "fbs_test").unwrap();
        std::fs::File::open("fbs_test/addressbook_generated.rs").unwrap();
    }
}
