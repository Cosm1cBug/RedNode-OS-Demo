# RedNode-OS Security — v0.34.0

> Security is the foundation, not a feature.

## Defense in Depth

```
Intent
  ↓ Attention Engine (score signal priority)
  ↓ Intent Clarification (vague → specific)
  ↓ Context Activation (relevant information only)
  ↓ Constitutional Check (7 immutable articles)
  ↓ Ethics Evaluation (privacy, safety, transparency)
  ↓ Uncertainty Assessment (confidence check)
  ↓ Multi-Agent Debate (multi-perspective consensus)
  ↓ Governance Policy Check (Block/Warn/Log/Approve)
  ↓ Risk Assessment (Low/Medium/High/Critical)
  ↓ Immune System Check (prompt injection, credential leaks)
  ↓ Trust Verification (source reliability score)
  ↓ Budget Check (resource cost within limits)
  ↓ Cognitive Load Check (capacity available)
  ↓ Approval Gate (High/Critical → human approval)
  ↓ Evolution Sandbox Check (if self-modification)
  ↓ Sandbox (firejail + seccomp)
  ↓ Execute
  ↓ Verification Engine (independently validate outcome)
  ↓ Explainability (record decision reasoning)
  ↓ Audit Log (SHA-256 hash chain)
  ↓ Reflection (success/failure analysis)
  ↓ Trust Update (source trust score adjusted)
  ↓ Cognitive Metrics Update
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

### 13. Evolution Sandbox (evolution_sandbox.rs)
Every self-modification is:
- Clone → Test (constitution + governance + ethics + budget)
- Benchmark → Validate → Approve → Deploy
- Prevents unsafe self-modification

### 14. Verification Engine (verification.rs)
Independently validates task outcomes:
- Checks claimed success vs actual output
- Tracks discrepancy rate and false positive rate
- Prevents silent failures

### 15. Uncertainty Engine (uncertainty.rs)
Knows when NOT to act:
- Proceed / DoubleCheck / AskUser / Defer / Abort
- Based on composite data + model + source confidence
- When uncertain, asks rather than guesses

## Privacy Guarantee

- **Zero cloud dependency** — all processing runs locally
- **Zero telemetry** — no data leaves the machine
- **Zero external analytics** — no tracking, no profiling
- **Constitutional protection** — Article 5 explicitly prohibits data exfiltration
- **Ethical value** — "Privacy over Convenience" as core operating principle
