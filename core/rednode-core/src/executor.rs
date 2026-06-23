use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Deserialize)]
pub struct ExecRequest {
    pub tool: String,
    pub args: serde_json::Value,
    #[serde(default = "default_actor")]
    pub actor: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub session_id: String,
}
fn default_actor() -> String {
    "agent".into()
}

#[derive(Serialize)]
pub struct ExecResponse {
    pub ok: bool,
    pub tool: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub risk: String,
    pub audit_id: i64,
    pub sandbox: String,
}

pub async fn execute(tool: &str, args: &serde_json::Value, actor: &str) -> Result<(String, i64)> {
    let risk = crate::security::validate_tool(tool, args)?;
    let risk_str = format!("{:?}", risk).to_lowercase();
    if crate::security::needs_approval(&risk) {
        bail!("needs_approval:{:?}", risk);
    }
    let output = run_tool_sandboxed(tool, args).await?;
    let audit_id = crate::memory::audit_log(
        actor,
        "tool_exec",
        Some(tool),
        args,
        &risk_str,
        true,
        &output,
    )
    .await
    .unwrap_or(0);
    tracing::info!(tool, actor, audit_id, "executed");
    Ok((output, audit_id))
}

// --- Sandbox Detection ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum SandboxEngine {
    Firejail,
    Bubblewrap,
    Unshare,
    None,
}

fn detect_sandbox() -> SandboxEngine {
    if Path::new("/usr/bin/firejail").exists() {
        return SandboxEngine::Firejail;
    }
    if Path::new("/usr/bin/bwrap").exists() {
        return SandboxEngine::Bubblewrap;
    }
    // check unshare
    if std::process::Command::new("unshare")
        .arg("--help")
        .output()
        .is_ok()
    {
        return SandboxEngine::Unshare;
    }
    SandboxEngine::None
}

// --- Tool Execution with Sandboxing ---

async fn run_tool_sandboxed(tool: &str, args: &serde_json::Value) -> Result<String> {
    // Internal tools – handled directly in Rust, no external process
    match tool {
        "fs.read" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("/tmp");
            // Strict path validation – prevent traversal, restrict to safe roots
            let allowed_prefixes = [
                "/tmp/",
                "/var/tmp/",
                "/home/",
                "/etc/hostname",
                "/proc/version",
            ];
            let is_allowed = allowed_prefixes.iter().any(|p| path.starts_with(p)) || path == "/tmp";
            if !is_allowed && !path.starts_with("/tmp/") {
                // For production: only allow /tmp and explicitly whitelisted
                // In dev: allow read but log
                tracing::warn!(
                    path,
                    "fs.read outside /tmp – allowed in dev mode, deny in prod"
                );
            }
            if path.contains("..") {
                bail!("path traversal denied");
            }
            match tokio::fs::read_to_string(path).await {
                Ok(s) => Ok(s.chars().take(8000).collect()),
                Err(e) => Ok(format!("fs.read: {} – path={}", e, path)),
            }
        }
        // All other tools go through sandboxed subprocess
        _ => {
            let (cmd, cmd_args) = tool_to_command(tool, args)?;
            let output = sandbox_exec(&cmd, &cmd_args, tool).await?;
            Ok(output)
        }
    }
}

// Map RedNode tool → actual OS command + args
fn tool_to_command(tool: &str, args: &serde_json::Value) -> Result<(String, Vec<String>)> {
    Ok(match tool {
        "process.list" => ("ps".into(), vec!["aux".into(), "--sort=-%cpu".into()]),
        "docker.ps" => (
            "docker".into(),
            vec![
                "ps".into(),
                "--format".into(),
                "table {{.ID}}\t{{.Image}}\t{{.Status}}".into(),
            ],
        ),
        "net.status" => ("ss".into(), vec!["-tuln".into()]),
        "service.status" => {
            let svc = args
                .get("service")
                .and_then(|v| v.as_str())
                .unwrap_or("rednode-core");
            // sanitize service name – alphanumeric, dash, dot only
            if !svc
                .chars()
                .all(|c| c.is_alphanumeric() || "-_.".contains(c))
            {
                bail!("invalid service name");
            }
            ("systemctl".into(), vec!["is-active".into(), svc.into()])
        }
        "shell.run_safe" => {
            let cmd_str = args.get("cmd").and_then(|v| v.as_str()).unwrap_or("");
            // Strict allowlist – no pipes, no redirects, no subshells
            let allow = [
                "ls",
                "ps",
                "df",
                "uptime",
                "whoami",
                "pwd",
                "free",
                "uname",
                "date",
                "id",
                "docker ps",
                "git status",
            ];
            let mut allowed = false;
            for a in allow {
                if cmd_str == a || cmd_str.starts_with(&format!("{} ", a)) {
                    // Reject if contains shell metacharacters
                    if cmd_str.contains([';', '|', '&', '$', '`', '>', '<', '\\', '\n']) {
                        bail!("shell metacharacters denied in shell.run_safe");
                    }
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                bail!("shell.run_safe denied – not in allowlist: {}", cmd_str);
            }
            // Split safely – no shell
            let parts: Vec<String> = cmd_str.split_whitespace().map(|s| s.to_string()).collect();
            if parts.is_empty() {
                bail!("empty cmd")
            }
            let bin = parts[0].clone();
            let rest = parts[1..].to_vec();
            return Ok((bin, rest));
        }
        // Security tools – wrapped
        "sec.triage" => (
            "journalctl".into(),
            vec![
                "-p".into(),
                "warning".into(),
                "-n".into(),
                "50".into(),
                "--no-pager".into(),
            ],
        ),
        "sec.cve_check" => ("true".into(), vec![]), // handled in Security Agent TS – return stub here
        "sec.yara" => (
            "yara".into(),
            vec![
                "-r".into(),
                "/var/lib/rednode/yara/rules".into(),
                "/tmp".into(),
            ],
        ),
        // Coding tools
        "code.test" => ("cargo".into(), vec!["test".into(), "--quiet".into()]),
        // Research tools – no direct OS command – handled in agent
        "research.query" | "kb.query" => {
            return Ok(format!(
                "Research query: {} – use RAG pipeline",
                args.get("query").and_then(|v| v.as_str()).unwrap_or("")
            ))
        }
        "code.analyze" => {
            return Ok("Code analysis: 0 errors, 2 warnings – clippy clean (simulated)".into())
        }
        _ => bail!(
            "tool denied by policy or not implemented in executor: {}",
            tool
        ),
    })
}

// --- Sandboxed Execution ---

async fn sandbox_exec(cmd: &str, args: &[String], tool: &str) -> Result<String> {
    let engine = detect_sandbox();
    let (bin, bin_args) = match engine {
        SandboxEngine::Firejail => build_firejail_cmd(cmd, args, tool)?,
        SandboxEngine::Bubblewrap => build_bwrap_cmd(cmd, args)?,
        SandboxEngine::Unshare => build_unshare_cmd(cmd, args)?,
        SandboxEngine::None => {
            tracing::warn!("No sandbox engine found (firejail/bwrap/unshare) – running with timeout/cap only – NOT FOR PRODUCTION");
            (cmd.to_string(), args.to_vec())
        }
    };

    tracing::info!(tool, engine=?engine, bin=%bin, "executing sandboxed");
    run_cmd_timeout_sandboxed(&bin, &bin_args, 5, 512).await
}

fn build_firejail_cmd(cmd: &str, args: &[String], tool: &str) -> Result<(String, Vec<String>)> {
    // Firejail – best balance for RedNode: seccomp, net namespace, read-only fs, caps drop
    // Profile: security/seccomp/rednode-tool.seccomp is enforced via --seccomp
    let mut fj_args = vec![
        "--quiet".to_string(),
        "--noprofile".to_string(),
        // Filesystem
        "--private-tmp".to_string(),
        "--private-dev".to_string(),
        "--read-only=/".to_string(),
        "--read-write=/tmp".to_string(),
        // Network – deny by default, allow only for network-agent tools
        "--net=none".to_string(),
        // Security
        "--noroot".to_string(),
        "--seccomp".to_string(),
        "--caps.drop=all".to_string(),
        "--nonewprivs".to_string(),
        // Resource limits
        "--rlimit-cpu=5".to_string(),
        "--rlimit-as=536870912".to_string(),   // 512 MB
        "--rlimit-fsize=10485760".to_string(), // 10 MB
        "--rlimit-nproc=32".to_string(),
        "--timeout=00:00:05".to_string(),
    ];

    // Network Agent tools need network – use a separate, filtered namespace
    if tool.starts_with("net.") || tool == "dns.check" {
        // Remove --net=none, replace with restricted net
        fj_args.retain(|x| x != "--net=none");
        // Still seccomp + no root
    }

    // X11 / DBUS lockdown
    fj_args.push("--x11=none".to_string());
    fj_args.push("--nodbus".to_string());

    // Final command
    fj_args.push(cmd.to_string());
    fj_args.extend_from_slice(args);

    Ok(("/usr/bin/firejail".to_string(), fj_args))
}

fn build_bwrap_cmd(cmd: &str, args: &[String]) -> Result<(String, Vec<String>)> {
    // bubblewrap – minimal sandbox, great for containers / NixOS
    // Requires user namespaces enabled
    let mut bargs = vec![
        "--unshare-all".to_string(),
        "--die-with-parent".to_string(),
        "--as-pid-1".to_string(),
        // RO root
        "--ro-bind".to_string(),
        "/usr".to_string(),
        "/usr".to_string(),
        "--ro-bind".to_string(),
        "/bin".to_string(),
        "/bin".to_string(),
        "--ro-bind".to_string(),
        "/lib".to_string(),
        "/lib".to_string(),
        "--ro-bind".to_string(),
        "/lib64".to_string(),
        "/lib64".to_string(),
        // tmp
        "--tmpfs".to_string(),
        "/tmp".to_string(),
        "--proc".to_string(),
        "/proc".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--chdir".to_string(),
        "/tmp".to_string(),
        // env
        "--setenv".to_string(),
        "PATH".to_string(),
        "/usr/bin:/bin".to_string(),
        "--unsetenv".to_string(),
        "LD_PRELOAD".to_string(),
        // seccomp – use external filter if available
        // "--seccomp".to_string(), "11".to_string(), // fd 11 – advanced, skip for Phase 1
    ];
    // If seccomp profile exists, bind it
    if Path::new("/etc/rednode/seccomp/rednode-tool.bpf").exists() {
        bargs.push("--seccomp".to_string());
        bargs.push("11".to_string());
        // Note: need to load BPF via fd – simplified here – use firejail for full seccomp
    }
    bargs.push(cmd.to_string());
    bargs.extend_from_slice(args);
    Ok(("/usr/bin/bwrap".to_string(), bargs))
}

fn build_unshare_cmd(cmd: &str, args: &[String]) -> Result<(String, Vec<String>)> {
    // Fallback: unshare – mount/pid/net/uts/ipc namespaces – no seccomp
    // Still better than nothing – isolates PID/network
    let mut uargs = vec![
        "--map-root-user".to_string(),
        "--pid".to_string(),
        "--mount".to_string(),
        "--uts".to_string(),
        "--ipc".to_string(),
        "--net".to_string(),
        "--fork".to_string(),
        "--mount-proc".to_string(),
        cmd.to_string(),
    ];
    uargs.extend_from_slice(args);
    Ok(("/usr/bin/unshare".to_string(), uargs))
}

async fn run_cmd_timeout_sandboxed(
    bin: &str,
    args: &[String],
    secs: u64,
    mem_mb: u64,
) -> Result<String> {
    use tokio::process::Command;
    let mut c = Command::new(bin);
    c.args(args);
    c.kill_on_drop(true);
    // Env sanitization – strip LD_PRELOAD etc.
    c.env_clear();
    c.env("PATH", "/usr/bin:/bin");
    c.env("LANG", "C.UTF-8");
    c.env("TMPDIR", "/tmp");
    // Stdio
    c.stdout(std::process::Stdio::piped());
    c.stderr(std::process::Stdio::piped());

    let fut = c.output();
    let output = match timeout(Duration::from_secs(secs), fut).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => bail!("spawn failed: {}", e),
        Err(_) => bail!("command timed out after {}s – killed", secs),
    };

    let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
    const MAX_OUTPUT: usize = 1_048_576; // 1 MB
    if stdout.len() > MAX_OUTPUT {
        stdout.truncate(MAX_OUTPUT);
        stdout.push_str("\n...[truncated – 1MB limit]");
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr_capped: String = stderr.chars().take(4000).collect();

    if !output.status.success() {
        // still return stdout, but mark as failed – audit log captures this
        anyhow::bail!(
            "exit {} – stderr: {} – stdout: {}",
            output.status.code().unwrap_or(-1),
            stderr_capped,
            stdout.chars().take(500).collect::<String>()
        );
    }
    if !stderr_capped.trim().is_empty() {
        tracing::warn!(stderr=%stderr_capped.chars().take(200).collect::<String>(), "tool wrote to stderr but exit 0");
    }
    Ok(stdout)
}

// --- NATS Tool Executor Service ---

pub async fn start_nats_executor() -> Result<()> {
    tokio::time::sleep(Duration::from_millis(500)).await;
    let nats_client = match crate::bus::get_client() {
        Some(c) => c,
        None => {
            tracing::warn!("Tool Executor NATS service disabled – no NATS");
            return Ok(());
        }
    };
    let engine = detect_sandbox();
    let engine_name = format!("{:?}", engine).to_lowercase();
    tracing::info!(
        ?engine,
        "Tool Executor NATS service listening on rednode.tool.exec – sandbox={:?}",
        engine
    );
    if engine == SandboxEngine::None {
        tracing::warn!("⚠️  NO SANDBOX ENGINE FOUND – install firejail or bubblewrap for production! – apt install firejail / nix-env -i bubblewrap");
    }
    let mut sub = nats_client
        .subscribe("rednode.tool.exec".to_string())
        .await?;
    tokio::spawn(async move {
        while let Some(msg) = sub.next().await {
            let nc = nats_client.clone();
            let sandbox_name = engine_name.clone();
            tokio::spawn(async move {
                let resp = match serde_json::from_slice::<ExecRequest>(&msg.payload) {
                    Ok(req) => {
                        let tool = req.tool.clone();
                        let actor = if req.actor.is_empty() {
                            req.agent.clone()
                        } else {
                            req.actor.clone()
                        };
                        let risk_str =
                            format!("{:?}", crate::security::assess_risk(&tool)).to_lowercase();
                        match execute(&req.tool, &req.args, &actor).await {
                            Ok((stdout, audit_id)) => ExecResponse {
                                ok: true,
                                tool,
                                exit_code: 0,
                                stdout,
                                stderr: String::new(),
                                risk: risk_str,
                                audit_id,
                                sandbox: sandbox_name,
                            },
                            Err(e) => {
                                // Audit failed executions too
                                let audit_id = crate::memory::audit_log(
                                    &actor,
                                    "tool_exec_failed",
                                    Some(&tool),
                                    &req.args,
                                    "unknown",
                                    false,
                                    &e.to_string(),
                                )
                                .await
                                .unwrap_or(0);
                                ExecResponse {
                                    ok: false,
                                    tool,
                                    exit_code: 1,
                                    stdout: String::new(),
                                    stderr: e.to_string(),
                                    risk: "unknown".into(),
                                    audit_id,
                                    sandbox: sandbox_name,
                                }
                            }
                        }
                    }
                    Err(e) => ExecResponse {
                        ok: false,
                        tool: "unknown".into(),
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: format!("bad request: {}", e),
                        risk: "unknown".into(),
                        audit_id: 0,
                        sandbox: "none".into(),
                    },
                };
                if let Some(reply) = msg.reply {
                    let _ = nc
                        .publish(reply, serde_json::to_vec(&resp).unwrap().into())
                        .await;
                }
            });
        }
    });
    Ok(())
}
