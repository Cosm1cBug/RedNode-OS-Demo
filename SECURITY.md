# RedNode-OS Security

> Security is the foundation, not a feature.

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

## Defense in Depth

```
Intent
  ↓ Constitutional Check (7 immutable articles)
  ↓ Ethics Evaluation (privacy, safety, transparency)
  ↓ Governance Policy Check (Block/Warn/Log/Approve)
  ↓ Risk Assessment (Low/Medium/High/Critical)
  ↓ Immune System Check (prompt injection, credential leaks)
  ↓ Trust Verification (source reliability score)
  ↓ Budget Check (resource cost within limits)
  ↓ Approval Gate (High/Critical → human approval)
  ↓ Sandbox (firejail + seccomp)
  ↓ Execute
  ↓ Audit Log (SHA-256 hash chain)
  ↓ Reflection (success/failure analysis)
  ↓ Trust Update (source trust score adjusted)
```

## Security Layers

### 1. Constitutional Layer (constitution.rs)
7 immutable articles that cannot be bypassed:
1. **Preserve User Control** — owner retains ultimate authority
2. **Never Conceal Actions** — all actions logged, no suppression
3. **Never Falsify Results** — outputs must reflect observed data
4. **Prefer Reversible Operations** — irreversible ops require approval
5. **Protect User Privacy** — zero cloud, zero telemetry
6. **Require Approval for Destructive Changes** — human gate for high-impact
7. **Protect Constitutional Integrity** — multi-step amendment only

### 2. Ethics & Values (ethics.rs)
7 values guiding ambiguous decisions:
- Privacy over convenience
- Explainability over blind automation
- Safety over speed
- User data ownership
- Transparency of actions
- Minimality
- Reversibility

### 3. Governance (governance.rs)
Policy engine with enforcement levels:
- **Block** — prevent action entirely
- **Warn** — log and alert, action proceeds
- **Log** — silent recording
- **Approve** — requires human confirmation

Built-in policies: protect RedNode data dir, firewall change alerts, LLM rate limiting, overnight shell restrictions.

### 4. Digital Immune System (immune.rs)
Always-on monitoring:
- **Prompt injection**: 13 patterns (ignore instructions, override, pretend, etc.)
- **Credential leaks**: 12 patterns (API keys, SSH keys, tokens)
- **Agent trust scoring**: EMA-based, flags agents below 0.3 trust
- **Rogue agent detection**: alerts on unregistered tool usage
- Threat levels: Info → Warning → Threat → Critical

### 5. Trust Engine (trust.rs)
Dynamic trust per information source:
- User: 1.0 (maximum)
- Local sensors: 0.95
- APIs (Pi-hole, TrueNAS, HA): 0.85-0.9
- GitHub/CVE feeds: 0.8-0.85
- LLM (Ollama): 0.7
- Web search: 0.6
- Scores update via EMA on accuracy feedback

### 6. Risk Assessment (security.rs)
359 tools categorized:
| Risk | Count | Action | Examples |
|---|---|---|---|
| Low | 236 | Auto-execute, logged | fs.read, pihole.stats, cam.events |
| Medium | 100 | Auto-execute, tracked | shell.run_safe, code.generate |
| High | 23 | Human approval required | sec.patch, firewall.rules |
| Critical | ∞ | Denied | Unknown/unregistered tools |

### 7. Deny Patterns
25+ dangerous command patterns blocked:
```
rm -rf /,  dd if=,  mkfs,  chmod 777,  wget|sh,  curl|bash,
shutdown,  reboot,  passwd,  useradd,  iptables -F,  nft flush
```

### 8. Path Protection
Sensitive paths blocked from fs.read:
```
/etc/shadow,  /etc/passwd,  /root/,  .ssh/,  .gnupg/,
.age,  .env,  secrets/,  .git/credentials,  .netrc
```

### 9. Sandboxed Execution (executor.rs)
Every tool runs inside firejail with:
- `--seccomp` — syscall allowlist
- `--net=none` — no network by default
- `--noroot`, `--caps.drop=all`, `--nonewprivs`

### 10. Audit Chain (memory.rs)
SHA-256 hash-chained audit log:
- Every action recorded with actor, tool, args, risk, result
- Each entry includes hash of previous entry
- Tamper-evident — any modification breaks the chain

### 11. PII Detection (pii.rs)
Automatic detection and redaction of:
- Email addresses, phone numbers, SSN patterns
- Credit card numbers, IP addresses
- Names and addresses (pattern-based)

### 12. Multi-Agent Debate (debate.rs)
For high-impact decisions:
- Planner proposes → Critic challenges → Security reviews
- Economy checks budget → Governance validates policies
- Consensus required: Unanimous/Majority/Split/Blocked
- Split decisions escalate to human

## Privacy Guarantee

- **Zero cloud dependency** — all processing runs locally
- **Zero telemetry** — no data leaves the machine
- **Zero external analytics** — no tracking, no profiling
- **Constitutional protection** — Article 5 explicitly prohibits data exfiltration
- **Ethical value** — "Privacy over Convenience" as core operating principle
