use failure::{bail, err_msg, format_err, Fallible};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
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
            eprintln!("Error code: {}", output.status.code().unwrap());
            eprintln!("{}", out);
            eprintln!("{}", err);
            bail!("Error: {:?}", self);
        }
        Ok(())
    }
}

/// Download and Build latest version of flatc
pub fn build_flatc() -> Fallible<PathBuf> {
    let work_dir = dirs::cache_dir()
        .ok_or(err_msg("Cannot get cache dir"))?
        .join("flatc-gen");
    if !work_dir.exists() {
        fs::create_dir_all(&work_dir)?;
    }

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
    fn flatc_gen() {
        // Be sure that this test runs at the top of this repo
        super::build_flatc().unwrap();
        super::flatc_gen("fbs/addressbook.fbs", "fbs_test").unwrap();
        std::fs::File::open("fbs_test/addressbook_generated.rs").unwrap();
    }
}
