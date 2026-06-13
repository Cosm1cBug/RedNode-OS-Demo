# RedNode-OS Security – Foundation, not feature

Zero Trust / Least Privilege / Memory Isolation / Agent Sandboxing / Cryptographic Verification / Secure-by-Default / Continuous Monitoring / Autonomous Defense

Pipeline:
Intention → Policy Engine → Risk Assessment → Approval → Sandbox (firejail/bubblewrap + seccomp) → Execute → Audit Log (hash-chained)

- RBAC: owner / agent / readonly
- Deny patterns: rm -rf /, dd, mkfs, fork bombs
- Network egress DENY – Network Agent proxies only
- Secrets: sops + age – never plaintext
- Updates: cosign signed, rollback capable
- Disk: LUKS FDE
- Dashboard: 127.0.0.1 + Tailscale only

Security Agent – Smart Security Mode:
- Hourly log triage (journalctl)
- Daily CVE sync, lynis, chkrootkit, YARA
- Auto-patch security updates with btrfs snapshot rollback
- eBPF + Falco real-time threat detection
- Self-healing: isolate → snapshot → remediate → verify

Threat Model: Prompt injection → tool args validated in Rust; LLM exfil → egress deny; Supply chain → cargo-audit / pnpm-audit CI; Physical theft → LUKS + TPM
