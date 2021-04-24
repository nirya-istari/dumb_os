#[macro_use] extern crate log;

use std::{env, fs, io, path::Path, process::{Command, ExitStatus}};

use bootloader_locator::{locate_bootloader, LocateError};

#[derive(Debug)]
enum BuildError {
    IoError {
        error: io::Error,
    },
    BuildError {
        exit_status: ExitStatus,
        build_command: Command,
    },
    BuilderError {
        exit_status: ExitStatus,
        builder_command: Command,
    }
}
impl From<io::Error> for BuildError {
    fn from(error: io::Error) -> Self {
        Self::IoError { error }
    }
}


fn main() -> Result<(), BuildError> {
    pretty_env_logger::init();
    

    let target = "x86_64-dumb_os";
    trace!("target = {}", target);
    let build_type = "debug";
    trace!("build_type = {}", build_type);

    let root = fs::canonicalize("../")?;
    trace!("root = {}", root.display());
    let kernel_path = root.join("kernel");
    trace!("kernel_crate_path = {}", kernel_path.display());
    let kernel_manifest = kernel_path.join("Cargo.toml");
    trace!("kernel_manifest = {}", kernel_manifest.display());

    let target_dir = root.join("target");
    trace!("target_dir = {}", target_dir.display());
    let out_dir = target_dir
        .join(target)
        .join(build_type);    
    trace!("out_dir = {}", out_dir.display());
    let kernel_binary =  out_dir
        .join("dumb_os");
    trace!("kernel_binary = {}", out_dir.display());
    
    println!("pwd: {}", env::current_dir()?.display());
    
    // Build the kernel
    let mut build_command = Command::new("cargo");
    build_command.current_dir(&kernel_path);
    build_command.arg("build");
    trace!("build_command: {:?}", &build_command);
    let exit_status = build_command.status()?;

    if !exit_status.success() {
        error!("exit_status: {:?}", exit_status);
        Err(BuildError::BuildError {
            build_command,
            exit_status,
        })?;
    }
    info!("exit_status: {:?}", exit_status);

    let bootloader_manifest_path = in_directory(&kernel_path, ||
        locate_bootloader("bootloader")
            .map_err(map_locate_error)
    )?;

    trace!("bootloader: {}", bootloader_manifest_path.display());
    let bootloader_path = bootloader_manifest_path.parent().expect("No parent dir");
    trace!("bootloader_path: {}", bootloader_path.display());

    let mut builder_command = Command::new("cargo");
    builder_command
        .arg("builder")
        .current_dir(&bootloader_path)
        .arg("--kernel-manifest").arg(&kernel_manifest)
        .arg("--kernel-binary").arg(&kernel_binary)
        .arg("--target-dir").arg(&target_dir)
        .arg("--out-dir").arg(&out_dir);
    
    info!("builder_command: {:?}", &builder_command);
    let exit_status = builder_command.status()?;
    
    if !exit_status.success() {
        error!("exit_status: {:?}", &exit_status);
        Err(BuildError::BuilderError {
            builder_command,
            exit_status,
        })?;
    }
    info!("exit_status: {:?}", &exit_status);

    Ok(())
}

fn map_locate_error(err: LocateError) -> io::Error {
    match err {
        err @ LocateError::MetadataInvalid => io::Error::new(io::ErrorKind::InvalidData, err),
        err @ LocateError::DependencyNotFound => io::Error::new(io::ErrorKind::NotFound, err),
        err @ LocateError::Metadata(_) => io::Error::new(io::ErrorKind::InvalidData, err),
    }
}


fn in_directory<P, F, R, E>(dir: P, f: F) -> Result<R, E>
    where P: AsRef<Path>,
          F: FnOnce() -> Result<R, E>,
          E: From<io::Error>
{
    let current = env::current_dir()?;
    env::set_current_dir(dir)?;
    let res = f();
    env::set_current_dir(&current)?;
    res
}