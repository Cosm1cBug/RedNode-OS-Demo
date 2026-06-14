# RedNode-OS — Windows Desktop App Build Guide

> Build the Tauri desktop app for Windows (also works on macOS and Linux).

---

## Prerequisites

1. **Node.js 22+** and **pnpm**
   ```powershell
   # Install Node.js from https://nodejs.org
   # Install pnpm:
   npm install -g pnpm
   ```

2. **Rust toolchain**
   ```powershell
   # Install rustup from https://rustup.rs
   # Then:
   rustup default stable
   rustup target add x86_64-pc-windows-msvc
   ```

3. **Visual Studio C++ Build Tools** (Windows only)
   ```powershell
   # Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
   # Select "Desktop development with C++" workload
   # This provides MSVC compiler needed by Tauri
   ```

4. **WebView2 Runtime** (usually pre-installed on Windows 10/11)
   ```powershell
   # Check if installed:
   # Settings → Apps → Microsoft Edge WebView2 Runtime
   # If missing: https://developer.microsoft.com/microsoft-edge/webview2/
   ```

---

## How the Desktop App Works

The Tauri app is a **thin native wrapper** around the Next.js web dashboard.

```
┌──────────────────────────────┐
│    RedNode Desktop (Tauri)   │
│    Native window (~8 MB)     │
│                              │
│  ┌────────────────────────┐  │
│  │   Next.js Dashboard    │  │
│  │   (loaded from         │  │
│  │    http://localhost:3000│  │
│  │    or bundled dist/)   │  │
│  └────────────────────────┘  │
│                              │
│  Rust backend:               │
│  - cns_health() command      │
│  - send_intent() command     │
│  - System tray (future)      │
└──────────────────────────────┘
         │
         │ HTTP/WS
         ▼
    RedNode CNS (:8787)
```

In **dev mode**: loads from `http://localhost:3000` (Next.js dev server)
In **production**: loads from bundled `dist/` folder

---

## Build Steps

### Development Mode (connects to running web dashboard)

```bash
# 1. Start the web dashboard first
cd interfaces/web
pnpm install
pnpm dev
# → http://localhost:3000

# 2. In another terminal, start the Tauri dev app
cd interfaces/desktop
pnpm install
pnpm tauri dev
# → Native window opens, loads localhost:3000
```

### Production Build (standalone installer)

```bash
# 1. Build the web dashboard
cd interfaces/web
pnpm install
pnpm build
# Creates .next/ output

# 2. Export as static files (for Tauri to bundle)
# Add to next.config.js:
#   output: 'export'
# Then:
pnpm build
# Creates out/ directory

# 3. Copy to Tauri's frontend directory
cp -r out/* ../desktop/dist/

# 4. Build the Tauri app
cd ../desktop
pnpm install
pnpm tauri build
# Creates:
#   src-tauri/target/release/bundle/msi/RedNode_0.2.0_x64.msi    (Windows installer)
#   src-tauri/target/release/bundle/nsis/RedNode_0.2.0_x64-setup.exe  (NSIS installer)
#   src-tauri/target/release/RedNode.exe  (standalone binary)
```

---

## Platform-Specific Builds

### Windows
```powershell
# Produces .msi and .exe installer
pnpm tauri build
# Output: src-tauri/target/release/bundle/msi/RedNode_0.2.0_x64.msi
```

### macOS
```bash
# Produces .dmg and .app bundle
pnpm tauri build
# Output: src-tauri/target/release/bundle/dmg/RedNode_0.2.0_x64.dmg
```

### Linux
```bash
# Produces .deb, .AppImage, .rpm
pnpm tauri build
# Output: src-tauri/target/release/bundle/deb/rednode_0.2.0_amd64.deb
#         src-tauri/target/release/bundle/appimage/RedNode_0.2.0_amd64.AppImage
```

---

## Configuration

The desktop app connects to your RedNode CNS server. Set the URL:

```bash
# Environment variable (before launching):
export REDNODE_CNS=http://10.0.50.10:8787

# Or via Tailscale:
export REDNODE_CNS=http://100.x.x.x:8787
```

The app reads `REDNODE_CNS` at startup and displays it in the window title.

---

## Tauri Config Reference

`interfaces/desktop/src-tauri/tauri.conf.json`:

```json
{
  "productName": "RedNode",
  "version": "0.2.0",
  "identifier": "os.rednode.desktop",
  "build": {
    "devUrl": "http://localhost:3000",       // Dev: load from Next.js
    "frontendDist": "../dist"                // Prod: load from bundled files
  },
  "app": {
    "windows": [{
      "title": "RedNode-OS",
      "width": 1200,
      "height": 800
    }]
  }
}
```

---

## App Size

| Platform | Installer Size | Installed Size |
|---|---|---|
| Windows (.msi) | ~5 MB | ~8 MB |
| macOS (.dmg) | ~4 MB | ~7 MB |
| Linux (.AppImage) | ~6 MB | ~8 MB |

The app is tiny because it's just a WebView wrapper — all the logic runs on your RedNode server.

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `pnpm tauri dev` shows blank window | Ensure `pnpm web` is running on port 3000 first |
| MSVC build errors on Windows | Install Visual Studio C++ Build Tools |
| `WebView2` not found | Install Microsoft Edge WebView2 Runtime |
| Can't connect to CNS | Set REDNODE_CNS env variable, check network/VPN |
| App crash on startup | Check `src-tauri/tauri.conf.json` — ensure devUrl or frontendDist is correct |
