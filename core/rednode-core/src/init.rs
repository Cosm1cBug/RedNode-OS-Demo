// RedNode-OS – Init – PID 1
// When RedNode IS the operating system.
//
// Compile: cargo build --bin rednode-init --features init
// Install: copy to /sbin/init on the RedNode-OS image
// Boot:    kernel param init=/sbin/rednode-init
//
// Responsibilities:
// 1. Mount virtual filesystems (/proc, /sys, /dev, /dev/pts, /dev/shm, /run, /tmp)
// 2. Set hostname
// 3. Bring up loopback network
// 4. Load environment from /var/lib/rednode/config/env
// 5. Start managed services in order: PostgreSQL → NATS → Ollama → RedNode CNS
// 6. Reap zombie processes (PID1 responsibility)
// 7. Handle SIGTERM/SIGINT → graceful shutdown
// 8. Handle SIGUSR1 → restart CNS
// 9. Watchdog: if CNS dies → restart (up to 5 times in 60s, then reboot)
// 10. SIGCHLD → reap children
//
// This is NOT a full init system like systemd. It supervises ONLY RedNode.
// NixOS services (Postgres, NATS, Ollama) are started via simple exec.

#![cfg(feature = "init")]

use nix::sys::reboot::{reboot, RebootMode};
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static RESTART_CNS: AtomicBool = AtomicBool::new(false);

// ─── Signal Handlers ───

extern "C" fn handle_sigterm(_: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

extern "C" fn handle_sigusr1(_: libc::c_int) {
    RESTART_CNS.store(true, Ordering::SeqCst);
}

fn setup_signals() {
    unsafe {
        let term_action = SigAction::new(
            SigHandler::Handler(handle_sigterm),
            SaFlags::SA_RESTART,
            SigSet::empty(),
        );
        let _ = sigaction(Signal::SIGTERM, &term_action);
        let _ = sigaction(Signal::SIGINT, &term_action);

        let usr1_action = SigAction::new(
            SigHandler::Handler(handle_sigusr1),
            SaFlags::SA_RESTART,
            SigSet::empty(),
        );
        let _ = sigaction(Signal::SIGUSR1, &usr1_action);

        // Ignore SIGCHLD — we reap manually
        // (don't use SA_NOCLDWAIT — we want to track exit codes)
    }
}

// ─── Filesystem Mounting ───

fn mount_all() -> anyhow::Result<()> {
    use nix::mount::{mount, MsFlags};

    let mounts = [
        (
            "/proc",
            "proc",
            "proc",
            MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID,
        ),
        (
            "/sys",
            "sysfs",
            "sysfs",
            MsFlags::MS_NODEV | MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID,
        ),
        ("/dev", "devtmpfs", "devtmpfs", MsFlags::MS_NOSUID),
        (
            "/run",
            "tmpfs",
            "tmpfs",
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        ),
        (
            "/tmp",
            "tmpfs",
            "tmpfs",
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        ),
    ];

    for (target, source, fstype, flags) in &mounts {
        let _ = std::fs::create_dir_all(target);
        let result = mount(Some(*source), *target, Some(*fstype), *flags, None::<&str>);
        match result {
            Ok(()) => println!("[init] mounted {}", target),
            Err(e) => eprintln!(
                "[init] mount {} failed: {} (may already be mounted)",
                target, e
            ),
        }
    }

    // /dev/pts (pseudo-terminals)
    let _ = std::fs::create_dir_all("/dev/pts");
    let _ = mount(
        Some("devpts"),
        "/dev/pts",
        Some("devpts"),
        MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        Some("gid=5,mode=620,ptmxmode=000"),
    );

    // /dev/shm (shared memory)
    let _ = std::fs::create_dir_all("/dev/shm");
    let _ = mount(
        Some("tmpfs"),
        "/dev/shm",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
        None::<&str>,
    );

    // Ensure /var/lib/rednode exists
    let _ = std::fs::create_dir_all("/var/lib/rednode");
    let _ = std::fs::create_dir_all("/var/lib/rednode/config");
    let _ = std::fs::create_dir_all("/var/lib/rednode/logs");

    Ok(())
}

// ─── Network ───

fn bring_up_lo() {
    let status = Command::new("/bin/ip")
        .args(["link", "set", "lo", "up"])
        .status();
    match status {
        Ok(s) if s.success() => println!("[init] loopback up"),
        _ => eprintln!("[init] failed to bring up loopback"),
    }
}

fn set_hostname() {
    let hostname = std::env::var("REDNODE_HOSTNAME").unwrap_or_else(|_| "rednode".into());
    let _ = std::fs::write("/proc/sys/kernel/hostname", &hostname);
    println!("[init] hostname: {}", hostname);
}

// ─── Environment Loading ───

fn load_environment() {
    // Load /var/lib/rednode/config/env if it exists
    let env_file = "/var/lib/rednode/config/env";
    if let Ok(content) = std::fs::read_to_string(env_file) {
        let mut loaded = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                // Don't override existing env vars (allow kernel cmdline overrides)
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                    loaded += 1;
                }
            }
        }
        println!("[init] loaded {} env vars from {}", loaded, env_file);
    }

    // Set defaults if not already set
    let defaults = [
        ("RUST_LOG", "info"),
        ("REDNODE_SENTIENCE", "on"),
        ("REDNODE_MODE", "os"),
        (
            "DATABASE_URL",
            "postgres://rednode:rednode@localhost/rednode",
        ),
        ("NATS_URL", "nats://127.0.0.1:4222"),
        ("QDRANT_URL", "http://127.0.0.1:6334"),
        ("OLLAMA_URL", "http://127.0.0.1:11434"),
    ];
    for (key, value) in defaults {
        if std::env::var(key).is_err() {
            std::env::set_var(key, value);
        }
    }
}

// ─── Service Management ───

struct ManagedService {
    name: &'static str,
    bin: String,
    args: Vec<String>,
    child: Option<Child>,
    required: bool, // if true, failure blocks boot
}

impl ManagedService {
    fn start(&mut self) -> bool {
        let result = Command::new(&self.bin)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn();

        match result {
            Ok(child) => {
                let pid = child.id();
                self.child = Some(child);
                println!("[init] started {} (PID {})", self.name, pid);
                true
            }
            Err(e) => {
                eprintln!("[init] failed to start {}: {}", self.name, e);
                false
            }
        }
    }

    fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,     // still running
                Ok(Some(_)) => false, // exited
                Err(_) => false,
            }
        } else {
            false
        }
    }

    fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            let pid = child.id();
            println!("[init] stopping {} (PID {})", self.name, pid);
            let _ = child.kill();
            let _ = child.wait();
            self.child = None;
        }
    }
}

// ─── Zombie Reaping ───

fn reap_zombies() {
    loop {
        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => break,
            Ok(status) => {
                if let WaitStatus::Exited(pid, code) = status {
                    if code != 0 {
                        eprintln!("[init] child PID {} exited with code {}", pid, code);
                    }
                }
            }
            Err(nix::errno::Errno::ECHILD) => break, // no children
            Err(_) => break,
        }
    }
}

// ─── Graceful Shutdown ───

fn shutdown(services: &mut [ManagedService], mode: RebootMode) {
    println!("\n[init] 🧠 RedNode-OS shutting down...\n");

    // Stop services in reverse order
    for svc in services.iter_mut().rev() {
        svc.stop();
    }

    // Sync filesystems
    println!("[init] syncing filesystems...");
    unsafe {
        libc::sync();
    }

    // Unmount /proc, /sys, etc.
    println!("[init] unmounting filesystems...");
    let _ = nix::mount::umount2("/dev/shm", nix::mount::MntFlags::MNT_DETACH);
    let _ = nix::mount::umount2("/dev/pts", nix::mount::MntFlags::MNT_DETACH);
    let _ = nix::mount::umount2("/tmp", nix::mount::MntFlags::MNT_DETACH);
    let _ = nix::mount::umount2("/run", nix::mount::MntFlags::MNT_DETACH);

    println!("[init] goodbye. the intelligence rests.\n");

    // Reboot or power off
    let _ = reboot(mode);
}

// ─── Main ───

#[cfg(feature = "init")]
fn main() -> anyhow::Result<()> {
    // Verify PID 1
    if std::process::id() != 1 {
        eprintln!("rednode-init: must run as PID 1");
        eprintln!("  For normal operation: cargo run --bin rednode-core");
        eprintln!("  For PID1 mode: set kernel param init=/path/to/rednode-init");
        std::process::exit(1);
    }

    println!();
    println!("  ╔══════════════════════════════════════════════╗");
    println!("  ║  🧠 RedNode-OS v0.3.1                       ║");
    println!("  ║  The computer becomes the intelligence.      ║");
    println!("  ║  PID 1 — init mode — all systems starting    ║");
    println!("  ╚══════════════════════════════════════════════╝");
    println!();

    // Phase 1: Early init
    setup_signals();
    mount_all()?;
    set_hostname();
    bring_up_lo();
    load_environment();

    println!("[init] early init complete — starting services...\n");

    // Phase 2: Service definitions
    let rednode_core_bin =
        std::env::var("REDNODE_CORE").unwrap_or_else(|_| "/usr/bin/rednode-core".into());

    let mut services: Vec<ManagedService> = vec![
        // PostgreSQL — must start first (memory depends on it)
        ManagedService {
            name: "postgresql",
            bin: std::env::var("POSTGRES_BIN").unwrap_or_else(|_| "/usr/bin/postgres".into()),
            args: vec!["-D".into(), "/var/lib/postgresql/data".into()],
            child: None,
            required: true,
        },
        // NATS — must start before CNS (agents depend on it)
        ManagedService {
            name: "nats-server",
            bin: std::env::var("NATS_BIN").unwrap_or_else(|_| "/usr/bin/nats-server".into()),
            args: vec!["-js".into(), "-sd".into(), "/var/lib/nats".into()],
            child: None,
            required: true,
        },
        // Ollama — LLM inference (CNS planner depends on it, but can start without)
        ManagedService {
            name: "ollama",
            bin: std::env::var("OLLAMA_BIN").unwrap_or_else(|_| "/usr/bin/ollama".into()),
            args: vec!["serve".into()],
            child: None,
            required: false,
        },
        // RedNode CNS — the brain
        ManagedService {
            name: "rednode-core",
            bin: rednode_core_bin,
            args: vec![],
            child: None,
            required: true,
        },
    ];

    // Phase 3: Start services in order
    for svc in services.iter_mut() {
        let ok = svc.start();
        if !ok && svc.required {
            eprintln!(
                "[init] CRITICAL: required service '{}' failed to start",
                svc.name
            );
            if svc.name == "rednode-core" {
                eprintln!("[init] CNS binary not found — check REDNODE_CORE env var");
            }
            // Wait 3s and retry once
            std::thread::sleep(Duration::from_secs(3));
            if !svc.start() {
                eprintln!(
                    "[init] FATAL: {} failed on retry — continuing in degraded mode",
                    svc.name
                );
            }
        }
        // Small delay between service starts to let ports bind
        std::thread::sleep(Duration::from_millis(500));
    }

    println!();
    println!("[init] 🧠 RedNode-OS — all services started — system ready");
    println!("[init] CNS: http://localhost:8787");
    println!("[init] Sentience Engine: ON");
    println!();

    // Phase 4: Supervision loop
    let mut cns_restart_count = 0u32;
    let mut cns_restart_window = Instant::now();

    loop {
        std::thread::sleep(Duration::from_millis(500));

        // Reap zombies (PID1 responsibility)
        reap_zombies();

        // Check for shutdown signal
        if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
            shutdown(&mut services, RebootMode::RB_POWER_OFF);
            std::process::exit(0);
        }

        // Check for CNS restart signal (SIGUSR1)
        if RESTART_CNS.load(Ordering::SeqCst) {
            RESTART_CNS.store(false, Ordering::SeqCst);
            println!("[init] SIGUSR1 received — restarting CNS...");
            if let Some(cns) = services.iter_mut().find(|s| s.name == "rednode-core") {
                cns.stop();
                std::thread::sleep(Duration::from_secs(1));
                cns.start();
            }
        }

        // Watchdog: check if CNS is alive
        if let Some(cns) = services.iter_mut().find(|s| s.name == "rednode-core") {
            if !cns.is_running() {
                eprintln!("[init] ⚠️ CNS died — restarting...");

                // Rate limit restarts: max 5 in 60 seconds
                if cns_restart_window.elapsed() > Duration::from_secs(60) {
                    cns_restart_count = 0;
                    cns_restart_window = Instant::now();
                }
                cns_restart_count += 1;

                if cns_restart_count > 5 {
                    eprintln!("[init] FATAL: CNS restarted 5 times in 60s — rebooting system");
                    shutdown(&mut services, RebootMode::RB_AUTOBOOT);
                    std::process::exit(1);
                }

                std::thread::sleep(Duration::from_secs(2));
                cns.start();
            }
        }

        // Check other services
        for svc in services.iter_mut() {
            if svc.name != "rednode-core" && svc.required && !svc.is_running() {
                eprintln!("[init] service '{}' died — restarting", svc.name);
                std::thread::sleep(Duration::from_secs(1));
                svc.start();
            }
        }
    }
}

#[cfg(not(feature = "init"))]
fn main() {
    eprintln!("rednode-init: compile with --features init and run as PID 1");
    eprintln!("  cargo build --bin rednode-init --features init");
    eprintln!("  For normal operation: cargo run --bin rednode-core");
    std::process::exit(1);
}
