// RedNode-OS – Init – PID 1
// When RedNode IS the operating system
// Compile with: cargo build --bin rednode-init --features init
//
// This becomes /sbin/init in the RedNode-OS image
// Responsibilities:
// - Mount /proc /sys /dev
// - Bring up loopback
// - Mount /var/lib/rednode (encrypted state – the Node identity)
// - Start RedNode CNS
// - Reap zombies
// - Handle poweroff/reboot signals
// - Watchdog – if CNS dies, restart or kexec rollback

#![cfg(feature = "init")]

use nix::sys::{reboot, wait::waitpid, signal::Signal};
use nix::unistd::Pid;
use std::process::{Command, Stdio};

fn mount_all() -> anyhow::Result<()> {
    use nix::mount::{mount, MsFlags};
    let _ = std::fs::create_dir_all("/proc");
    let _ = std::fs::create_dir_all("/sys");
    let _ = std::fs::create_dir_all("/dev");
    let _ = std::fs::create_dir_all("/run");
    let _ = std::fs::create_dir_all("/var/lib/rednode");
    let _ = mount(Some("proc"), "/proc", Some("proc"), MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID, None::<&str>);
    let _ = mount(Some("sysfs"), "/sys", Some("sysfs"), MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID, None::<&str>);
    let _ = mount(Some("devtmpfs"), "/dev", Some("devtmpfs"), MsFlags::MS_NOSUID, Some("mode=0755"));
    Ok(())
}

fn bring_up_lo() {
    let _ = Command::new("/bin/ip").args(["link", "set", "lo", "up"]).status();
}

fn start_cns() -> anyhow::Result<std::process::Child> {
    // In production image, rednode-core is at /nix/store/.../bin/rednode-core
    // or /usr/bin/rednode-core
    let bin = std::env::var("REDNODE_CORE").unwrap_or("/usr/bin/rednode-core".into());
    let child = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdout::inherit())
        .env("RUST_LOG", "info")
        .env("REDNODE_MODE", "os")
        .spawn()?;
    Ok(child)
}

fn reap_zombies() {
    loop {
        match waitpid(Pid::from_raw(-1), Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
            Ok(nix::sys::wait::WaitStatus::StillAlive) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
}

#[cfg(feature = "init")]
fn main() -> anyhow::Result<()> {
    // Are we PID1?
    if std::process::id() != 1 {
        eprintln!("rednode-init must run as PID 1");
        std::process::exit(1);
    }

    println!("\n🧠 RedNode-OS – booting – intelligence is the operating layer\n");

    mount_all()?;
    bring_up_lo();

    println!("[init] RedNode CNS starting…");
    
    loop {
        let mut cns = match start_cns() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[init] failed to start CNS: {} – retry in 3s", e);
                std::thread::sleep(std::time::Duration::from_secs(3));
                continue;
            }
        };

        // Wait + reap loop
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            reap_zombies();
            
            match cns.try_wait() {
                Ok(Some(status)) => {
                    eprintln!("[init] CNS exited with {} – restarting in 3s", status);
                    break;
                }
                Ok(None) => continue,
                Err(e) => {
                    eprintln!("[init] wait error: {}", e);
                    break;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

#[cfg(not(feature = "init"))]
fn main() {
    eprintln!("rednode-init: compile with --features init and run as PID 1");
}
