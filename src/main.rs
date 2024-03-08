use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::mem::take;
use std::process::{exit, Child, Command, Stdio};
use std::sync::{Arc, Mutex};

fn main() {
    // All of this logic is needed to correctly wipe the tempdir when ctrlc is hit.

    let tempdir = tempfile::tempdir().expect("cargo-brr: Failed to create tempdir!");
    let path = tempdir.path().to_path_buf();
    let tempdir = Arc::new(Mutex::new(Some(tempdir)));
    let ctrlc_tempdir = tempdir.clone();

    let mut child_handle = run_cargo(path);
    let mut child_stdout = child_handle.stdout.take().unwrap();
    let child_handle = Arc::new(Mutex::new(child_handle));
    let ctrlc_child_handle = child_handle.clone();

    ctrlc::set_handler(move || {
        ctrlc_child_handle
            .lock()
            .unwrap()
            .kill()
            .expect("cargo-brr: Failed to kill cargo child process!");

        close_tempdir(&ctrlc_tempdir);
        println!("");
        exit(1);
    })
    .expect("Error setting Ctrl-C handler");

    while child_stdout.read_exact(&mut [0; 1024]).is_ok() {}
    std::thread::sleep(std::time::Duration::from_millis(1000));
    child_handle
        .lock()
        .unwrap()
        .wait()
        .expect("cargo-brr: Faield to wait on cargo child process!");

    close_tempdir(&tempdir);
}

fn run_cargo(path: std::path::PathBuf) -> Child {
    let mut temp_cargo_toml = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.join("Cargo.toml"))
        .expect("cargo-brr: Failed to open temp Cargo.toml!");

    temp_cargo_toml
        .write_all(include_bytes!("./ProjectCargo.toml"))
        .expect("cargo-brr: Failed to write to temp Cargo.toml!");

    let mut temp_main = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.join("main.rs"))
        .expect("cargo-brr: Failed to open temp main.rs!");

    temp_main
        .write_all(b"fn main() {}")
        .expect("cargo-brr: Failed to write to temp main.rs!");

    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("cargo-brr: Failed to spawn cargo child process!")
}

fn close_tempdir(tempdir: &Mutex<Option<tempfile::TempDir>>) {
    let tempdir = take(&mut *tempdir.lock().unwrap()).unwrap();
    tempdir
        .close()
        .expect("cargo-brr: Failed to delete tempdir!");
}
