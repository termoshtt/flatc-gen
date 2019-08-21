use log::info;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    thread::sleep,
    time::Duration,
};

fn check_output(output: &Output, command_name: &str) {
    if !output.status.success() {
        let out = String::from_utf8(output.stdout.clone()).expect("Failed to parse output");
        let err = String::from_utf8(output.stderr.clone()).expect("Failed to parse error output");
        eprintln!("=== {} output ===", command_name);
        eprintln!("{}", out);
        eprintln!("{}", err);
        panic!(
            "{} failed with error code: {}",
            command_name,
            output.status.code().unwrap()
        );
    }
}

/// Download and Build latest version of flatc
fn build_flatc() -> PathBuf {
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
                eprintln!("Waiting lock of {}, {:?}", lock_file.display(), err);
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
        let st = Command::new("git")
            .args(&["clone", "http://github.com/google/flatbuffers"])
            .current_dir(&work_dir)
            .status()
            .expect("Git is not installed");
        if !st.success() {
            panic!("Git clone of google/flatbuffers failed");
        }
    }

    // Build flatbuffers
    let output = Command::new("cmake")
        .args(&["-Bbuild", "-H."])
        .current_dir(&fbs_repo)
        .output()
        .expect("cmake not found");
    check_output(&output, "cmake");

    let output = Command::new("cmake")
        .args(&["--build", "build", "--target", "flatc"])
        .current_dir(&fbs_repo)
        .output()
        .expect("cmake not found");
    check_output(&output, "cmake");

    fbs_repo.join("build/flatc")
}

pub fn flatc_gen(path: impl AsRef<Path>, out_dir: impl AsRef<Path>) {
    let path = path.as_ref();
    if !path.exists() {
        panic!("Flatbuffer file '{}' does not exist.", path.display());
    }

    // Generate Rust code from FlatBuffer definitions
    let flatc = build_flatc();
    let st = Command::new(flatc)
        .args(&["-r", "-o"])
        .arg(out_dir.as_ref())
        .arg("-b")
        .arg(&path)
        .status()
        .expect("flatc command failed");
    if !st.success() {
        panic!("flatc failed: {}", st.code().expect("No error code"));
    }
}
