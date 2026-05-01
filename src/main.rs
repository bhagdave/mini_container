extern crate nix;
extern crate clap;
extern crate libc;

use nix::sched::{unshare, CloneFlags};
use nix::sys::wait::waitpid;
use nix::unistd::{execvp, fork, ForkResult};
use nix::mount::{mount, MsFlags};
use clap::{App, Arg, SubCommand};
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() {

    let matches = App::new("Simple container CLI")
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs a command in an isolated container")
                .arg(Arg::with_name("COMMAND").required(true).index(1))
                .arg(Arg::with_name("ARGS").multiple(true).index(2)),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploys files or directories to the container root")
                .arg(Arg::with_name("PATHS").required(true).multiple(true).index(1)),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("run") {
        let cmd = matches.value_of("COMMAND").unwrap();
        let args: Vec<&str> = matches.values_of("ARGS").unwrap_or_default().collect();
        unsafe {
            run_container(cmd, args);
        }
    } else if let Some(matches) = matches.subcommand_matches("deploy") {
        let paths: Vec<&str> = matches.values_of("PATHS").unwrap().collect();
        deploy_container(paths);
    } else {
        println!("No subcommand was used");
    }

}



fn deploy_container(paths: Vec<&str>) {
    let newroot = Path::new("newroot");
    std::fs::create_dir_all(newroot).expect("Failed to create newroot");

    for path in paths {
        let source = Path::new(path);
        let deploy_name = source.file_name().expect("Failed to determine source name");
        let deploy_path = newroot.join(deploy_name);
        copy_into_root(source, &deploy_path);
    }
}

fn copy_into_root(source: &Path, destination: &Path) {
    if source.is_dir() {
        fs::create_dir_all(destination).expect("Failed to create destination directory");

        for entry in fs::read_dir(source).expect("Failed to read source directory") {
            let entry = entry.expect("Failed to read directory entry");
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            copy_into_root(&source_path, &destination_path);
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directory");
        }

        fs::copy(source, destination).expect("Failed to deploy file");
        println!("Deployed {:?} to {:?}", source, destination);
    }
}

unsafe fn run_container(cmd: &str,args: Vec<&str>) {
    println!("Running command: {} with args: {:?}", cmd, args);
    match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            waitpid(child, None).expect("Failed to wait for child");
        }
        Ok(ForkResult::Child) => {
            let c_cmd = CString::new(cmd).expect("Failed to convert cmd to CString");
            let c_args: Vec<CString> = args.iter().map(|arg| CString::new(*arg).expect("Failed to convert arg to CString")).collect();
            let c_args_refs: Vec<&std::ffi::CStr> = c_args.iter().map(AsRef::as_ref).collect();

            unshare(CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS).expect("Failed to unshare");

            let current_dir = std::env::current_dir().unwrap();
            setup_rootfs(&format!("{}/newroot", current_dir.display()));

            execvp(&c_cmd, &c_args_refs).expect("Failed to execvp");
        }
        Err(err) => {
            panic!("Failed to fork: {:?}", err);
        }
    }
}

fn setup_rootfs(newroot: &str) {
    std::env::set_current_dir(newroot).expect("Failed to set current dir");
    let new_root_c = CString::new(newroot).expect("Failed to convert newroot to CString");
    unsafe{
        if libc::chroot(new_root_c.as_ptr()) != 0 {
            panic!("Failed to chroot");
        }   
    }
    std::env::set_current_dir("/").expect("Failed to set current dir");
    fs::create_dir_all("/proc").expect("Failed to create /proc");
    if !is_proc_mounted() {
        mount(Some("proc"), "/proc", Some("proc"), MsFlags::MS_NOSUID | MsFlags::MS_NODEV, None::<&str>).expect("Failed to mount /proc");
    }

}

fn is_proc_mounted() -> bool {
    let file = match File::open("/proc/mounts") {
        Ok(file) => file,
        Err(_) => return false,
    };

    let reader = BufReader::new(file);

    for line in reader.lines() {
        if let Ok(l) = line {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.len() > 1 && parts[1] == "/proc" {
                return true;
            }
        }
    }
    false
}

