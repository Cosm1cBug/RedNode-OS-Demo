# RedNode-OS Security — Foundation, Not Feature

## Security Pipeline

```
Intent → Policy Engine → Risk Assessment → Approval Gate → Sandbox → Execute → Audit Log (SHA-256 chain)
```

## Risk Levels — 114 Tools

| Risk | Action | Count | Examples |
|---|---|---|---|
| **Low** | Auto-execute, logged | 60+ | fs.read, process.list, pihole.stats, cam.events, social.feed |
| **Medium** | Auto-execute, logged, tracked | 30+ | shell.run_safe, code.generate, social.post, browser.download |
| **High** | **Requires human approval** | 8 | sec.harden_ssh, sec.patch, firewall.rules, browser.fill |
| **Critical** | **Denied** (unknown tools) | ∞ | Anything not in the registry |

## Deny Patterns (25+)

```
rm -rf /,  dd if=,  mkfs,  :(){ :|:& };,  chmod 777 /,  wget|sh,  curl|bash,
shutdown,  reboot,  passwd,  useradd,  iptables -F,  nft flush,  ...
```

## Sandboxed Execution

Every tool runs inside firejail/bubblewrap with:
- `--seccomp` — syscall allowlist (60+ dangerous syscalls blocked)
- `--net=none` — no network access by default
- `--noroot`, `--caps.drop=all`, `--nonewprivs`
- `--rlimit-cpu=5`, `--rlimit-as=512MB`, `--rlimit-fsize=10MB`
- stdout capped at 1 MB, 5-second timeout, `kill_on_drop`

## Audit Log

Every action is recorded in PostgreSQL with a **SHA-256 hash chain**:
```
hash[n] = SHA-256(hash[n-1] + actor + action + tool + args + risk)
```
Tamper-evident: modifying any entry breaks the chain for all subsequent entries.

## Authentication

- **API**: Bearer token middleware with constant-time comparison
- **Mobile**: Biometric (fingerprint/FaceID) for High/Critical approvals
- **Signal Bot**: Owner-only — rejects messages from non-owner numbers
- **Dashboard**: VLAN-isolated (only trusted devices on VLAN 10 can reach)

## Threat Intelligence

- **CVE scanning**: Real dpkg/rpm/nix inventory against NVD database (syncs every 24h)
- **Threat feeds**: abuse.ch (Feodo, SSL BL, URLhaus) + AlienVault OTX + Emerging Threats
- **Auto-blocking**: IOC IPs → pfSense firewall alias. Malicious domains → Pi-hole deny list.
- **Falco eBPF**: Real-time syscall monitoring (tails Falco log, fallback to journalctl)
- **Auto-patching**: btrfs/zfs snapshot → apt/dnf upgrade → verify → rollback on failure

## Network Security

- **VLAN isolation**: Cameras on VLAN 30 (zero internet), management on VLAN 50
- **pfSense firewall**: Zero open inbound ports. Network Agent manages rules.
- **WireGuard/Tailscale**: Remote access via VPN only
- **DNS**: Pi-hole blocks ads, trackers, and malicious domains for all VLANs
- **Egress**: Default DENY. Network Agent proxies allowed connections.

## Secret Management

- `sops + age` — encrypted at rest, never plaintext
- Environment variables — 80+ vars in `.env.example`, never hardcoded
- Android Keystore — hardware-backed AES-256-GCM for mobile credentials
- Signal: E2EE from phone to server via signal-cli

## Disk Encryption

- **LUKS FDE** — full disk encryption with passphrase
- **TPM2 auto-unlock** — sealed to PCR 0,2,7 (optional)
- **Btrfs snapshots** — pre-patch snapshots for rollback

## Threat Model

| Threat | Mitigation |
|---|---|
| Prompt injection | Tool args validated in Rust before execution — LLM cannot bypass |
| LLM exfiltration | Egress deny by default — LLM has no network access |
| Supply chain | cargo-audit / pnpm-audit in CI |
| Physical theft | LUKS + TPM — disk unreadable without passphrase |
| Compromised IoT | VLAN isolation + Pi-hole + pfSense auto-blocking |
| Camera feed exfil | Cameras on VLAN 30 — zero internet access |

## Code Quality

- **Zero unsafe Rust** in entire codebase
- **Zero SQL injection** — all 19+ queries use parameterized `$1, $2...`
- **Zero hardcoded secrets** — 80+ vars via environment
- **35 integration tests** — security, events, planner, audit chain, RAG
