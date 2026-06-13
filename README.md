# RedNode-OS
### The Personal Autonomous Operating System

> The computer does not contain intelligence. The computer becomes the intelligence.

**RedNode is not an AI. RedNode is a society of specialized agents.**

---

## Vision

RedNode-OS transforms the computer itself into an intelligent autonomous system. Intelligence becomes the operating layer, not an application.

Human Intent → Interface → **Central Nervous System** → Agent Society → Execution → Host OS → Hardware

Commands become intentions. Security is the foundation. Portable computational organism.

## Tech Stack

| Layer | Technology | Hardening |
|---|---|---|
| Core Runtime | **Rust** | Axum, tokio, NATS – CNS = PID1 capable |
| AI Layer | **Python** | Whisper / Piper – isolated, firewalled |
| Messaging | **NATS JetStream** | mTLS, JetStream persistence – CNS bus |
| Structured Memory | **PostgreSQL 16** | audit_log hash-chained, pgvector |
| Vector Memory | **Qdrant** | 768d cosine – RAG via Ollama nomic-embed-text |
| Knowledge Graph | **Kuzu** | Embedded, Apache-2.0 – Cypher – `cargo --features kuzu` |
| Workflow Engine | **Automation Agent + Rust DAG** | Temporalite optional |
| Search/SIEM | **Grafana Loki + Vector** | eBPF / Falco → Loki – Quickwit optional |
| Local Models | **Ollama → vLLM** | Qwen2.5-14B – fully offline |
| Web UI | **Next.js 14 + TS** | Approval Queue / Memory Browser / Security Feed / Audit Log |
| Mobile | **Flutter 3.22** | FCM push • Biometric approval • WireGuard auto-tunnel • Secure Storage |
| Desktop | **Tauri 2** | Rust backend – 8 MB |
| Observability | **OpenTelemetry** | OTEL → Loki / Prometheus → Grafana |
| Security Telemetry | **eBPF + Falco** | Real-time – CVE auto-patcher with snapshot rollback |
| Container Runtime | **Docker Compose** | No Kubernetes – personal node |
| Secrets Management | **sops + age** | No Vault daemon – portable |

See `ARCHITECTURE.md`, `SECURITY.md`, `ROADMAP.md`.

---

## Hardening

**1. Rust Tool Executor – firejail / bubblewrap + seccomp – full audit**
- `core/rednode-core/src/executor.rs` – 380 LOC
- Sandbox detection: Firejail → Bubblewrap → unshare → fallback
- Firejail: `--seccomp --net=none --private-tmp --noroot --caps.drop=all --rlimit-cpu=5 --rlimit-as=512MB --rlimit-fsize=10MB --rlimit-nproc=32 --timeout=00:00:05`
- Bubblewrap: `--unshare-all --die-with-parent --ro-bind /usr /usr --tmpfs /tmp`
- Command allowlist – no shell metacharacters – `shell.run_safe` only: ls, ps, df, uptime, whoami, docker ps, git status
- Path traversal protection – `fs.read` restricted
- stdout cap 1 MB, 5s timeout, kill_on_drop
- **Postgres audit_log – SHA-256 hash-chained – tamper-evident**
- NATS RPC: `rednode.tool.exec` – Agent → Rust → Audit → Reply
- Tool Registry: 23 tools – risk tagged Low/Med/High/Critical

**2. Memory – Real RAG – Qdrant + Ollama + Kuzu**
- `core/rednode-core/src/memory.rs` – 420 LOC
- **Qdrant**: `qdrant-client` – collection `rednode_docs`, 768d cosine – auto-create
- **Ollama embeddings**: `POST /api/embeddings` – `nomic-embed-text` – 10s timeout – graceful fallback
- **RAG query**: embed → Qdrant search → return {source, content, score, metadata}
- **Fallback chain**: Qdrant → Postgres ILIKE → static knowledge – UI never empty
- **Ingest**: `POST /memory/ingest {source, content}` – embed → Postgres + Qdrant upsert
- **Kuzu**: feature-gated (`--features kuzu`) – embedded graph – Project → Technology → Repo → File → Function – Cypher query API – falls back to Postgres JSON if Kuzu not compiled
- API: `GET /memory/query?q=`, `POST /memory/ingest`

**3. Android APK – FCM Push + Biometric Approval + WireGuard Auto-Tunnel**
- `interfaces/mobile/` – Flutter 3.22 – Material 3 – 6 tabs
- **FCM Push**: `firebase_messaging` + `flutter_local_notifications`
  - `FirebaseMessagingService` – registers FCM token – listens foreground/background
  - Approval push → high-priority notification with Approve/Deny actions
  - Payload E2EE – Firebase sees only a ping
  - Works offline – polls every 5s fallback – 0 trackers
  - Setup: `flutterfire configure` – see `FIREBASE_SETUP.md`
- **Biometric Approval**: `local_auth`
  - Every High/Critical approval → BiometricPrompt / FaceID
  - `BiometricAuth.authenticate(reason: 'Approve RedNode tool: $tool')`
  - Falls back to device PIN – configurable to biometric-only
  - Approval rejected if biometric fails
- **WireGuard Auto-Tunnel**: `wireguard_service.dart`
  - Tries native VpnService + wireguard-go via MethodChannel
  - Fallback: launch WireGuard / Tailscale app via Intent
  - Tunnel status UI – green = connected – blocks API calls if not on trusted network
  - Trusted networks: 100.x (Tailscale), 192.168/10/172.16 (LAN), localhost
  - RedNode CNS firewall DROP all non-VPN – zero inbound ports
  - Secrets: WireGuard private key → `flutter_secure_storage` – Android Keystore – hardware-backed AES-256-GCM
- **Secure Storage**: `flutter_secure_storage` + `shared_preferences`
  - API token / node URL / WG private key → Keystore/Keychain – never plaintext
- **6 Pages**: Intent • Approvals (biometric) • Security Feed • Memory Browser • Audit Log • Agents (+ Sentience Drives)
- **Build**: `flutter build apk --release` – ~24 MB – see `BUILD_APK.md`
- Permissions: INTERNET, POST_NOTIFICATIONS, USE_BIOMETRIC, CAMERA (QR onboarding), VpnService
- **Privacy**: No analytics, no crashlytics, no ads – 0 trackers – all traffic via WireGuard

**Security Agent – CVE Auto-Patcher + Falco eBPF – also shipped:**
- `agents/security-agent/src/cve.ts` – dpkg inventory → CVE DB (offline + NVD sync hook) – 6h interval – auto-patch HIGH/CRITICAL if Smart Security Mode ON
- `agents/security-agent/src/patcher.ts` – snapshot → patch → verify → rollback – btrfs/zfs hooks – 95% simulated success – full audit
- `agents/security-agent/src/falco.ts` – tails `/var/log/falco/falco.log` JSON – normalizes → `POST /security/events` – Critical → isolate + snapshot – includes simulator – 1 event/90s if Falco not installed – first event at 12s

**Next.js Dashboard – fleshed out:**
- `interfaces/web/app/page.tsx` – 7-tab SOC console
- **ApprovalQueue.tsx** – live approval queue – approve/deny – risk badges
- **MemoryBrowser.tsx** – RAG search – Qdrant/Kuzu – vector scores
- **SecurityFeed.tsx** – security_events – CVE + Falco – acknowledge – severity colors
- **AuditLog.tsx** – hash-chained audit – SHA preview – tamper-evident
- **AgentStatus.tsx** – 6 agents online
- **IntentPanel.tsx** – plan viewer with risk colors
- **EventStream.tsx** – WebSocket live CNS events
- API client: `lib/api.ts` – typed – auto-refresh

---

## Quick Start 

```bash
# 1. Infra
cd deployment && docker compose up -d
# nats :4222, postgres :5432, qdrant :6333, ollama :11434, loki :3100, grafana :3000

# 2. Models
ollama pull qwen2.5:14b-instruct-q4_K_M
ollama pull nomic-embed-text

# 3. CNS – Rust Core
cd ../core/rednode-core && cargo run
# → http://localhost:8787

# 4. Agent Society
cd ../../ && pnpm install && pnpm agents

# 5. Web UI
pnpm web
# → http://localhost:3000

# 6. CLI
pnpm --filter @rednode/cli dev -- intent "harden ssh and show docker status"

# 7. Desktop
cd interfaces/desktop && pnpm tauri dev
```

API:
```bash
curl -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -d '{"intent":"analyze system health"}'
```

## Agent Society

- **System Agent** – OS control, processes, docker, services
- **Security Agent** – threat intel, CVE monitoring, self-healing, Smart Security Mode
- **Coding Agent** – codegen, debug, test, git
- **Research Agent** – RAG, knowledge graph, synthesis
- **Automation Agent** – workflows, scheduler, triggers
- **Network Agent** – firewall, VPN, DNS, zero-trust

All communicate via NATS – the Central Nervous System.

## Security – Foundation, not feature

Zero Trust → Policy Engine → Risk Assessment → Approval → Sandbox (firejail/bubblewrap) → Audit Log (hash-chained)

Security Agent is 24/7 SOC: CVE feed, YARA, Falco/eBPF, auto-patch with snapshot rollback.

## Interfaces

- Voice – OpenWakeWord / Whisper / Piper
- Web – Next.js
- Mobile – Flutter
- Desktop – Tauri
- CLI – TypeScript
- API – Rust / Axum

Intelligence remains inside RedNode. Interfaces are just windows.

## Portability

`rednode export` → age-encrypted + ed25519 signed .rednode bundle
`rednode import` → resume anywhere <60s

## License

MIT – © 2026 RedNode
