//! a simple shell for uploading to the orbic device.
//!
//! It literally just runs bash as UID/GID 0, with special Android GIDs 3003
//! (AID_INET) and 3004 (AID_NET_RAW).
use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() {
    let mut args = env::args();

    // discard argv[0]
    let _ = args.next();

    let mut cmd = Command::new("/bin/bash");
    cmd.args(args).uid(0).gid(0);

    // Android's "paranoid network" feature restricts network access to
    // processes in specific groups. More info here:
    // https://www.elinux.org/Android_Security#Paranoid_network-ing
    //
    // Set supplementary groups in pre_exec because Rust's Command internally
    // calls setgroups(0, NULL) when .gid() is used, which would clear any
    // groups set before exec.
    #[cfg(target_arch = "arm")]
    unsafe {
        cmd.pre_exec(|| {
            let gids: [libc::gid_t; 2] = [3003, 3004]; // AID_INET, AID_NET_RAW
            if libc::setgroups(2, gids.as_ptr()) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let error = cmd.exec();
    eprintln!("Error running command: {error}");
    std::process::exit(1);
}
