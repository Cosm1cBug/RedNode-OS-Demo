# RedNode-OS — Tools Expansion Map & Backpropagation

---

## Part 1: Tools That Can Be Added To Each Agent

Current: **359 tools** across 17 agent types. Below are **287 additional tools** that can be added.

---

### 🔧 System Agent (currently 33 → can reach 55)

**Hardware Monitoring:**
- `sys.gpu_status` — GPU utilization, VRAM usage, temperature (nvidia-smi / rocm-smi)
- `sys.battery` — Laptop battery level, charge rate, health cycle count
- `sys.pci_devices` — List PCI devices (lspci)
- `sys.block_devices` — List block devices with sizes and mount points (lsblk)
- `sys.kernel_modules` — List loaded kernel modules (lsmod)
- `sys.dmesg_errors` — Recent kernel errors only (dmesg --level=err,warn)

**Process Management:**
- `sys.kill_process` — Kill a process by PID or name (high risk)
- `sys.top_cpu` — Top 10 CPU consumers right now
- `sys.top_mem` — Top 10 memory consumers right now
- `sys.open_files` — List open files by a process (lsof)
- `sys.open_ports` — List all listening ports (ss -tlnp)

**NixOS Specific:**
- `sys.nix_generations` — List all NixOS generations with sizes
- `sys.nix_gc` — Run Nix garbage collection (medium risk)
- `sys.nix_diff` — Diff two NixOS generations
- `sys.nix_why` — Show why a package is in the closure (nix-store --query --referrers)
- `sys.nix_size` — Show closure size of a package

**System Info:**
- `sys.hostname` — Show/set hostname
- `sys.timezone` — Show/set timezone
- `sys.locale` — Show locale settings
- `sys.env_vars` — List environment variables (filtered for safety)
- `sys.disk_usage` — df -h output
- `sys.swap_status` — Swap usage and configuration

---

### 🛡️ Security Agent (currently 28 → can reach 48)

**Authentication & Access:**
- `sec.ssh_sessions` — List active SSH sessions
- `sec.ssh_keys` — List authorized SSH keys for all users
- `sec.login_history` — Show recent login/logout events (last)
- `sec.failed_logins` — Show failed login attempts (lastb)
- `sec.sudoers_check` — Audit sudoers configuration
- `sec.user_audit` — List all users, check for suspicious accounts

**Network Security:**
- `sec.open_ports_audit` — Compare open ports against expected baseline
- `sec.tls_config_check` — Check TLS configuration quality for a service
- `sec.dns_poisoning_check` — Verify DNS responses against known-good
- `sec.arp_spoofing_check` — Detect ARP spoofing on local network

**File Integrity:**
- `sec.file_integrity` — Check hash of critical system files against baseline
- `sec.suid_scan` — Find all SUID/SGID binaries (privilege escalation risk)
- `sec.world_writable` — Find world-writable files in sensitive directories
- `sec.tmp_scan` — Scan /tmp for suspicious files
- `sec.crontab_audit` — Audit all user crontabs for suspicious entries

**Threat Response:**
- `sec.quarantine` — Move suspicious file to quarantine directory
- `sec.incident_report` — Generate incident report from security events
- `sec.block_country` — Block all IPs from a country via pfSense (high risk)
- `sec.threat_summary` — AI-generated daily threat summary
- `sec.attack_surface` — Map the system's attack surface (exposed services, ports, users)

---

### 🌐 Network Agent (currently 30 → can reach 48)

**Diagnostics:**
- `net.latency_test` — Measure latency to multiple endpoints simultaneously
- `net.packet_loss` — Extended packet loss test to a target
- `net.tcp_connect` — Test TCP connection to host:port with timing
- `net.ssl_handshake` — Test and time SSL/TLS handshake to a server
- `net.http_headers` — Fetch and display HTTP response headers for a URL
- `net.curl_timing` — Detailed HTTP request timing (DNS, connect, TLS, TTFB)

**pfSense Integration:**
- `net.pfsense_status` — pfSense system status overview
- `net.pfsense_interfaces` — List pfSense interfaces with status
- `net.pfsense_dhcp_static` — Add/remove static DHCP mapping
- `net.pfsense_aliases` — Manage pfSense aliases (IP lists)
- `net.pfsense_logs` — Query pfSense firewall logs

**Network Management:**
- `net.dns_flush` — Flush local DNS cache
- `net.hostname_resolve` — Resolve hostname to IP and reverse
- `net.mac_lookup` — Look up MAC address vendor (OUI database)
- `net.ip_geolocation` — Geolocate an IP address (local GeoIP database)
- `net.subnet_calc` — Calculate subnet details (broadcast, range, mask)
- `net.port_check` — Check if a specific port is open on a host
- `net.network_map` — Generate a network topology map from ARP + DNS data

---

### 🏗️ Infrastructure Agent (currently 22 → can reach 35)

**Pi-hole Advanced:**
- `pihole.flush_logs` — Flush Pi-hole query logs
- `pihole.long_term_stats` — Query long-term statistics database
- `pihole.network_overview` — Network-wide device DNS activity
- `pihole.update_check` — Check if Pi-hole update is available
- `pihole.tail_log` — Real-time tail of Pi-hole query log

**Docker Advanced:**
- `docker.stats` — Live resource usage per container (CPU, RAM, Net I/O)
- `docker.inspect` — Inspect container configuration
- `docker.volumes` — List Docker volumes with sizes
- `docker.networks` — List Docker networks
- `docker.compose_status` — Show docker-compose project status
- `docker.compose_pull` — Pull latest images for compose project
- `docker.health` — Check all container health statuses
- `docker.exec` — Execute command inside a container (high risk)

---

### 💾 Storage Agent (currently 24 → can reach 38)

**TrueNAS Advanced:**
- `nas.jail_list` — List jails/VMs
- `nas.services` — List TrueNAS services and status
- `nas.network` — TrueNAS network configuration
- `nas.cert_list` — List TrueNAS SSL certificates
- `nas.update_check` — Check for TrueNAS updates
- `nas.reboot` — Reboot TrueNAS (high risk)
- `nas.shutdown` — Shutdown TrueNAS (critical risk)

**File Operations:**
- `nas.directory_size` — Calculate directory size on NAS
- `nas.recent_files` — List recently modified files
- `nas.large_files` — Find largest files on a share
- `nas.orphan_snapshots` — Find orphaned snapshots consuming space
- `nas.backup_schedule` — Show/configure backup schedules
- `nas.integrity_check` — Verify file checksums against stored hashes
- `nas.usage_trend` — Storage usage trend over time (growing/shrinking)

---

### 📹 Surveillance Agent (currently 26 → can reach 40)

**Camera Management:**
- `cam.reboot` — Reboot a camera via ONVIF (medium risk)
- `cam.settings` — Get/set camera settings (exposure, resolution, etc.)
- `cam.firmware_check` — Check camera firmware version
- `cam.rtsp_test` — Test RTSP stream connectivity and latency
- `cam.bandwidth_usage` — Bandwidth consumed per camera stream
- `cam.storage_usage` — Disk space used by recordings per camera

**AI Detection:**
- `cam.object_count` — Count specific objects in a camera's view right now
- `cam.heatmap` — Generate activity heatmap from detection history
- `cam.dwell_time` — Average dwell time of people in a zone
- `cam.unusual_activity` — Detect unusual patterns (wrong time, wrong zone)
- `cam.package_detect` — Detect package deliveries at door
- `cam.pet_detect` — Filter events for pet detections only

**Presence Advanced:**
- `presence.schedule` — Expected presence schedule (work hours, sleep, away)
- `presence.geofence` — Geofence-based presence (phone location via HA)

---

### 📧 Communications Agent (currently 18 → can reach 30)

**Email Advanced:**
- `email.folders` — List email folders/labels
- `email.mark_read` — Mark email as read/unread
- `email.star` — Star/flag an email
- `email.forward` — Forward an email to another address
- `email.template` — Use/manage email templates
- `email.bounce_check` — Check if sent emails bounced

**Calendar Advanced:**
- `calendar.upcoming` — Next N upcoming events
- `calendar.recurring` — List/manage recurring events
- `calendar.shared` — View shared calendars
- `calendar.travel_time` — Estimate travel time between calendar events
- `calendar.daily_summary` — AI-generated daily calendar summary

**Notifications:**
- `notify.preferences` — Get/set notification channel preferences per category

---

### 📝 Productivity Agent (currently 23 → can reach 38)

**Notes Advanced:**
- `notes.template` — Create note from template
- `notes.archive` — Archive old notes
- `notes.recent` — Recently modified notes
- `notes.word_count` — Word count for a note
- `notes.backlinks` — Find all notes linking to a given note

**Tasks Advanced:**
- `tasks.overdue` — List overdue tasks
- `tasks.today` — Tasks due today
- `tasks.week` — Tasks due this week
- `tasks.delegate` — Assign task to another person (future multi-user)
- `tasks.estimate` — Set time estimate for a task
- `tasks.time_log` — Log time spent on a task

**Habits Advanced:**
- `habits.list` — List all tracked habits
- `habits.stats` — Habit completion statistics over time
- `habits.best_day` — Best performing day of the week for habits

---

### 🎵 Media Agent (currently 21 → can reach 33)

**Photo Advanced:**
- `photo.albums` — List/create photo albums
- `photo.metadata` — Read EXIF metadata from a photo
- `photo.location_map` — Map photos by GPS location
- `photo.timeline` — Photo timeline by date
- `photo.slideshow` — Generate slideshow from an album

**Music Advanced:**
- `music.now_playing` — Currently playing track info
- `music.queue` — View/manage playback queue
- `music.similar` — Find similar songs/artists
- `music.scrobble` — Log playback to local history

**Video Advanced:**
- `video.chapters` — Extract chapters from video
- `video.subtitle` — Extract/add subtitles (ffmpeg)
- `video.compress` — Compress video to reduce size

---

### 🏠 Home Agent (currently 19 → can reach 32)

**Device Management:**
- `home.firmware_updates` — Check for device firmware updates
- `home.zigbee_devices` — List Zigbee devices and signal quality
- `home.zwave_devices` — List Z-Wave devices and status
- `home.bluetooth_devices` — List Bluetooth devices

**Automation Advanced:**
- `home.create_automation` — Create HA automation via API (high risk)
- `home.trigger_automation` — Manually trigger an automation
- `home.automation_history` — Automation execution history
- `home.condition_check` — Check if automation conditions are met

**Climate Advanced:**
- `home.thermostat_schedule` — View/set thermostat schedule
- `home.weather_forecast` — Local weather forecast from HA
- `home.humidity` — Indoor humidity readings
- `home.air_quality` — Air quality sensor data
- `home.window_status` — Window open/close sensor status

---

### 💻 Coding Agent (currently 21 → can reach 35)

**Code Intelligence:**
- `code.explain` — AI explanation of a code snippet
- `code.optimize` — AI-suggested optimizations
- `code.translate` — Translate code between languages
- `code.document` — Generate documentation from code
- `code.type_check` — Run TypeScript type checker
- `code.benchmark` — Run performance benchmarks

**Git Advanced:**
- `git.stash` — Stash/unstash changes
- `git.cherry_pick` — Cherry-pick a commit
- `git.blame` — Show who changed each line (git blame)
- `git.tags` — List/create git tags
- `git.remote` — Manage git remotes
- `git.revert` — Revert a commit (high risk)
- `git.issues` — List GitHub issues
- `git.releases` — List/create GitHub releases

---

### 🌐 Browser Agent (currently 13 → can reach 22)

- `browser.cookies` — List cookies for a domain
- `browser.headers` — Show full request/response headers
- `browser.redirect_trace` — Follow redirect chain and show each hop
- `browser.sitemap` — Parse sitemap.xml and list all URLs
- `browser.robots` — Parse robots.txt rules
- `browser.lighthouse` — Run Lighthouse audit (performance, accessibility, SEO)
- `browser.form_detect` — Detect and list forms on a page
- `browser.meta_tags` — Extract meta tags (OG, Twitter, SEO)
- `browser.broken_links` — Scan page for broken links

---

### 📱 Social Agent (currently 16 → can reach 25)

- `social.audience_insights` — AI analysis of audience demographics
- `social.content_calendar` — Generate content calendar suggestions
- `social.trend_detect` — Detect trending topics in your niche
- `social.sentiment` — Sentiment analysis of comments/replies
- `social.competitor` — Track competitor social activity
- `social.report` — Generate weekly social media report
- `social.image_generate` — Generate social media image (if local Stable Diffusion available)
- `social.caption` — AI-generate image caption
- `social.optimal_length` — Suggest optimal post length per platform

---

### 🧠 Learning Agent (currently 17 → can reach 27)

- `learn.curriculum` — Generate a learning curriculum for a topic
- `learn.quiz` — Generate quiz questions from learned material
- `learn.explain` — Explain a concept in simple terms using LLM
- `learn.compare_models` — Compare performance of different LLM models on RedNode tasks
- `learn.error_analysis` — Analyze why specific intents fail
- `learn.optimize_prompts` — Suggest improvements to system prompts
- `learn.tool_usage_stats` — Which tools are used most/least
- `learn.feedback_loop` — Record user feedback on tool outputs for future improvement
- `learn.capability_matrix` — Generate matrix of what RedNode can vs can't do
- `learn.documentation_gaps` — Find undocumented features or tools

---

### 📱 Signal Bot (currently 6 → can reach 12)

- `signal.schedule_message` — Schedule a message for later delivery
- `signal.auto_reply` — Set auto-reply when owner is away
- `signal.message_history` — Search past Signal messages
- `signal.reaction` — React to a message
- `signal.typing_indicator` — Show/hide typing indicator
- `signal.profile` — Get/set Signal profile (name, avatar)

---

### ⚙️ Automation Agent (currently 15 → can reach 25)

- `workflow.import` — Import workflow from JSON/YAML file
- `workflow.export` — Export workflow to JSON/YAML
- `workflow.clone` — Clone an existing workflow
- `workflow.test` — Dry-run a workflow without executing
- `workflow.stats` — Execution statistics for a workflow
- `schedule.next` — Show when each schedule will next trigger
- `trigger.cron` — Create cron-based trigger
- `trigger.http` — Trigger workflow via HTTP endpoint
- `trigger.email` — Trigger workflow when email arrives matching pattern
- `trigger.device` — Trigger workflow when network device appears/disappears

---

## Summary

| Agent | Current | Can Add | Total Possible |
|---|---|---|---|
| System | 33 | 22 | 55 |
| Security | 28 | 20 | 48 |
| Network | 30 | 18 | 48 |
| Infrastructure | 22 | 13 | 35 |
| Storage | 24 | 14 | 38 |
| Surveillance | 26 | 14 | 40 |
| Communications | 18 | 12 | 30 |
| Productivity | 23 | 15 | 38 |
| Media | 21 | 12 | 33 |
| Home | 19 | 13 | 32 |
| Coding | 21 | 14 | 35 |
| Browser | 13 | 9 | 22 |
| Social | 16 | 9 | 25 |
| Learning | 17 | 10 | 27 |
| Signal Bot | 6 | 6 | 12 |
| Automation | 15 | 10 | 25 |
| **TOTAL** | **359** | **287** | **646** |

---

## Part 2: Backpropagation — What It Is and Does It Matter to RedNode?

### What Is Backpropagation?

Backpropagation (short for "backward propagation of errors") is the core algorithm used to **train neural networks**. It's how AI models like Qwen, Llama, GPT, etc. learn from data.

Here's how it works in simple terms:

```
1. FORWARD PASS:
   Input data → flows through the neural network → produces an output
   Example: "check camera events" → model predicts → [cam.events, surveillance-agent]

2. COMPARE:
   Compare the model's output to the correct answer
   Error = model_output - correct_answer
   Example: model said [cam.status] but correct was [cam.events] → error = wrong tool

3. BACKWARD PASS (Backpropagation):
   The error flows BACKWARD through the network
   At each layer, the algorithm calculates:
   "How much did THIS layer's weights contribute to the error?"
   Then it nudges each weight slightly to reduce the error

4. REPEAT:
   Do this millions of times with different examples
   → The model gradually learns the correct patterns
```

**Analogy:** Imagine you're throwing darts at a target. Each throw (forward pass), you see where the dart landed (output). You compare to the bullseye (correct answer). Then you adjust your aim (backpropagation). Over thousands of throws, you get more accurate.

### Does Backpropagation Matter to RedNode?

**Short answer: Not directly, but indirectly YES — in two specific ways.**

#### 1. RedNode Does NOT Run Backpropagation (Inference Only)

RedNode uses **pre-trained** models (Qwen 2.5 via Ollama). When you say "check camera events," the model runs a **forward pass only** — it generates a plan. It does NOT learn from this interaction. The weights don't change. This is called **inference**.

```
RedNode TODAY:
  Intent → Qwen 2.5 (frozen weights) → Plan → Execute
  ↑ model never changes from usage
```

#### 2. Backpropagation IS Used During Fine-Tuning

When you fine-tune the model using `docs/guides/FINETUNE-LLM.md`, backpropagation is what happens under the hood:

```
Fine-tuning (LoRA):
  Your 200+ intent-plan pairs → backpropagation → LoRA adapter weights
  → These small adapter weights (50-200 MB) customize the model for YOUR patterns
  → Plan accuracy jumps from 90% → 97%+
```

So backpropagation runs **offline** during fine-tuning, not during live operation.

#### 3. What RedNode Uses INSTEAD of Backpropagation (at Runtime)

RedNode has its own learning mechanisms that don't use backpropagation:

| Mechanism | How It Works | Similar To |
|---|---|---|
| **Pattern Promotion** | Counts how often entities/patterns appear → promotes frequent ones | Frequency-based learning |
| **Memory Consolidation** | JUDGE phase scores memories → CONSOLIDATE prunes weak ones | Spaced repetition |
| **Predictive Intent** | Tracks daily usage patterns → suggests actions proactively | Statistical prediction |
| **Self-Evolution** | Discovers tools → generates handlers → registers them | Meta-learning |
| **Audit Mining** | Learning Agent extracts patterns from audit log | Pattern recognition |
| **Knowledge Graph** | Builds entity-relationship connections across facts | Symbolic reasoning |

These are **non-neural** learning methods. They don't need backpropagation because they don't train neural networks — they learn through **statistics, rules, and pattern matching**.

#### 4. Could RedNode Use Backpropagation in the Future?

Yes, in two scenarios:

**Scenario A: Continuous Online Learning**
Instead of fine-tuning once, RedNode could continuously adjust the LoRA adapter after every interaction:
```
Intent → Plan → Execute → Result
                ↓
         Was the plan correct?
                ↓
         If no → backpropagation → update LoRA adapter
                ↓
         Next intent → slightly better plan
```
This is called **online learning** or **continual learning**. It's technically possible but risky — bad feedback can degrade the model. RedNode's current approach (collect data → fine-tune periodically) is safer.

**Scenario B: Training Specialized Small Models**
Instead of using a general 7B parameter model for everything, RedNode could train tiny specialized models:
- A 100M parameter model JUST for tool selection (which of 359 tools to pick)
- A 50M parameter model JUST for risk assessment
- A 50M parameter model JUST for intent classification

These would train using backpropagation on RedNode's own data, run in <1ms (vs 200ms for Qwen), and be more accurate for their specific tasks.

### Summary

| Question | Answer |
|---|---|
| What is backpropagation? | Algorithm that trains neural networks by flowing errors backward to adjust weights |
| Does RedNode use it at runtime? | **No** — Qwen runs inference only (forward pass, frozen weights) |
| Does RedNode use it at all? | **Yes** — during LoRA fine-tuning (offline, manual) |
| What does RedNode use instead? | Pattern promotion, memory consolidation, audit mining, self-evolution, knowledge graphs |
| Should RedNode add it? | Future: possibly for continuous online learning or training specialized micro-models |
| Is it urgent? | **No** — RedNode's current learning mechanisms are effective without it |
