
#[macro_use]
extern crate log;


use std::{env, io, path::Path};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use bootloader_locator::{locate_bootloader, LocateError};
use env::args;
use serde::Deserialize;
use semver::Version;

#[derive(Debug, Deserialize)]
struct CargoConfig {
    cargo: PathBuf,
    cargo_home: PathBuf,
    cargo_manifest_dir: PathBuf,
    cargo_pkg_authors: String,
    cargo_pkg_description: String,
    cargo_pkg_homepage: String,
    cargo_pkg_license: String,
    cargo_pkg_license_file: String,
    cargo_pkg_name: String,
    cargo_pkg_repository: String,
    cargo_pkg_version: Version,
    cargo_pkg_version_major: u64,
    cargo_pkg_version_minor: u64,
    cargo_pkg_version_patch: u64,
    cargo_pkg_version_pre: String,
}

#[tokio::main]
async fn main() -> Result<(), i32> {
    pretty_env_logger::init();

    let args: Vec<_> = args().collect();
    let target_binary = PathBuf::from(args.get(1).expect("Missing argument. Expected executable path"));
    let target_binary_name: String = target_binary.file_name().unwrap().to_str().unwrap().to_string();
    let is_test = target_binary.parent().unwrap().ends_with("deps");

    let target_dir = target_binary.ancestors().find(|p| p.ends_with("target"))
        .expect("binary is not in target folder");

    let env: CargoConfig = envy::from_env().expect("Incorrect environment vars");

    trace!("args: {:?}", args);
    trace!("env: {:#?}", env);    

    let target = "x86_64-dumb_os";
    trace!("target = {}", target);    

    trace!("kernel_crate_path = {}", env.cargo_manifest_dir.display());
    let kernel_manifest = env.cargo_manifest_dir.join("Cargo.toml");
    trace!("kernel_manifest = {}", kernel_manifest.display());

    let out_dir = target_binary.parent().unwrap();
    trace!("target_binary = {}", target_binary.display());    


    let bootloader_manifest_path = in_directory(&env.cargo_manifest_dir, || {
        locate_bootloader("bootloader").map_err(map_locate_error)
    }).expect("Failed to find bootloader crate");

    trace!("bootloader_manifest_path: {}", bootloader_manifest_path.display());
    let bootloader_path = bootloader_manifest_path.parent().expect("No parent dir");
    trace!("bootloader_path: {}", bootloader_path.display());

    let mut builder_command = Command::new("cargo");
    builder_command
        .arg("builder")
        .current_dir(&bootloader_path)
        .arg("--kernel-manifest")
        .arg(&kernel_manifest)
        .arg("--kernel-binary")
        .arg(&target_binary)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--out-dir")
        .arg(&out_dir);

    info!("builder_command: {:?}", &builder_command);
    let exit_status = builder_command.status().await
        .expect("Builder failed");

    if !exit_status.success() {
        error!("exit_status: {:?}", &exit_status);
        panic!("builder failed");        
    }
    info!("exit_status: {:?}", &exit_status);
    
    let kernel_image = out_dir.join(format!("boot-bios-{}.img", target_binary_name));
    std::fs::metadata(&kernel_image).expect("uefi file does not exist");

    

    // let ovmf_vars = env.cargo_manifest_dir.join("ovmf_vars.fd");    

    let mut qemu_command = Command::new("qemu-system-x86_64");
    qemu_command
        .arg("-enable-kvm")        
        .arg("-machine").arg("q35")
        .arg("-drive").arg(format!("if=ide,format=raw,file={}", kernel_image.display()))
        .arg("-serial").arg("stdio")
        .arg("-s");
    
    let status = if is_test {
        qemu_command
            .arg("-display").arg("none")
            .arg("-device").arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
            // Otherwise qemu hangs around after panic is thrown
            .kill_on_drop(true);
        timeout(Duration::from_secs(5), qemu_command.status())
            .await
            .expect("qemu timed out")
            
    } else {
        qemu_command.status().await                    
    }.expect("failed to start qemu");

    info!("qemu command: {:?}", &qemu_command);
   

    let status_code = status.code().expect("Error getting status code");
    // This is qemu's successful exit status when using isa-debug-exit.    
    if is_test && status_code != 33 {
        error!("Command failed {}", status);
        return Err(status_code);
    } else if !is_test && status_code != 0 {
        error!("Command failed {}", status);
        return Err(status_code);
    }
    return Ok(())
}

fn map_locate_error(err: LocateError) -> io::Error {
    match err {
        err @ LocateError::MetadataInvalid => io::Error::new(io::ErrorKind::InvalidData, err),
        err @ LocateError::DependencyNotFound => io::Error::new(io::ErrorKind::NotFound, err),
        err @ LocateError::Metadata(_) => io::Error::new(io::ErrorKind::InvalidData, err),
    }
}

fn in_directory<P, F, R, E>(dir: P, f: F) -> Result<R, E>
where
    P: AsRef<Path>,
    F: FnOnce() -> Result<R, E>,
    E: From<io::Error>,
{
    let current = env::current_dir()?;
    env::set_current_dir(dir)?;
    let res = f();
    env::set_current_dir(&current)?;
    res
}
