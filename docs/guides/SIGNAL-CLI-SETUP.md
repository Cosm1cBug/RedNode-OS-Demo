# RedNode-OS — Signal CLI Setup Guide

> Step-by-step: connect RedNode to Signal messenger for E2EE notifications and commands.

---

## What You Need

- A **spare phone number** for the bot (can be a VoIP number or old SIM)
- A phone that can receive one SMS for verification
- RedNode machine with internet access

---

## Step 1: Install signal-cli

### On NixOS (recommended):
```bash
nix-env -iA nixpkgs.signal-cli
```

### On other Linux:
```bash
# Download latest release
wget https://github.com/AsamK/signal-cli/releases/latest/download/signal-cli-0.13.4-Linux.tar.gz
tar xf signal-cli-*.tar.gz
sudo mv signal-cli-0.13.4/bin/signal-cli /usr/local/bin/
sudo mv signal-cli-0.13.4/lib/ /usr/local/lib/signal-cli/
```

### Verify installation:
```bash
signal-cli --version
# Should print: signal-cli 0.13.x
```

---

## Step 2: Register the Bot's Phone Number

```bash
# Replace +1BOTNUMBER with the spare phone number
signal-cli -u +1BOTNUMBER register

# You'll receive an SMS with a verification code
# Enter it:
signal-cli -u +1BOTNUMBER verify CODE
# Example: signal-cli -u +919876543210 verify 123-456
```

### If registration fails:
```bash
# Try with captcha (Signal sometimes requires it)
# 1. Open https://signalcaptchas.org/registration/generate.html in a browser
# 2. Solve the captcha
# 3. Copy the signalcaptcha://... URL
signal-cli -u +1BOTNUMBER register --captcha "signalcaptcha://signal-recaptcha-v2...."
```

---

## Step 3: Configure RedNode

Edit `/var/lib/rednode/source/.env`:
```bash
nano /var/lib/rednode/source/.env
```

Set these values:
```
# Signal Bot
SIGNAL_CLI_PATH=/usr/bin/signal-cli        # or wherever signal-cli is installed
SIGNAL_BOT_NUMBER=+1BOTNUMBER              # the number you just registered
SIGNAL_OWNER_NUMBER=+1YOURPERSONALNUMBER   # YOUR personal Signal number
SIGNAL_POLL_INTERVAL=3000                  # check for messages every 3 seconds
```

---

## Step 4: Test the Connection

```bash
# Send a test message from the bot to yourself
signal-cli -u +1BOTNUMBER send -m "Hello from RedNode! 🧠" +1YOURPERSONALNUMBER
```

Check your Signal app — you should receive the message.

---

## Step 5: Start the Signal Bot

```bash
# Option A: via start-all.sh (starts everything)
./scripts/start-all.sh start

# Option B: manually
cd /var/lib/rednode/source
pnpm --filter @rednode/signal-bot dev
```

---

## Step 6: Chat with RedNode from Signal

Open Signal on your phone and message the bot number:

```
You:     status
RedNode: 🧠 RedNode-OS — System Health: all services running...

You:     check cameras
RedNode: 📹 Camera Events: 3 person detections in last hour...

You:     goodnight
RedNode: 🌙 Goodnight workflow: DNS strict mode, cameras armed...
```

### Available commands from Signal:
- `status` — system health
- `help` — list all commands
- `goodnight` — run goodnight workflow
- `morning` — run morning briefing
- Any natural language intent: "check disk space", "show DNS stats", etc.

---

## Security Notes

- **Owner-only:** The bot ONLY responds to `SIGNAL_OWNER_NUMBER`. All other messages are rejected.
- **E2E Encrypted:** Signal Protocol encrypts all messages. RedNode never sees plaintext in transit.
- **Local processing:** signal-cli runs on your machine. No cloud Signal servers see your commands.
- **No message storage:** Messages are processed and discarded. Only the audit log records the intent and result.

---

## Troubleshooting

### "signal-cli: command not found"
```bash
which signal-cli
# If not found, check your PATH or reinstall
```

### "Unregistered number"
```bash
# Re-register
signal-cli -u +1BOTNUMBER register
signal-cli -u +1BOTNUMBER verify NEW_CODE
```

### "Bot doesn't respond"
```bash
# Check if signal-bot is running
pnpm --filter @rednode/signal-bot dev

# Check logs
journalctl -u rednode-signal-bot -f  # if running as systemd service
# or
cat /var/lib/rednode/logs/signal-bot.log
```

### "Messages from wrong number rejected"
This is correct behavior. Only `SIGNAL_OWNER_NUMBER` can interact with the bot. Change the number in `.env` if needed.
