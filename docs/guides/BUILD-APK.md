# RedNode-OS — Android APK Build Guide

> Build the Flutter mobile app into an APK you can install on your Android phone.

---

## Prerequisites

1. **Flutter SDK** 3.22+ installed
   ```bash
   # Install Flutter
   # Option A: FVM (recommended)
   dart pub global activate fvm
   fvm install 3.22.0
   fvm use 3.22.0

   # Option B: Direct
   git clone https://github.com/flutter/flutter.git -b stable
   export PATH="$PATH:$(pwd)/flutter/bin"

   # Verify
   flutter doctor
   ```

2. **Android SDK** — installed via Android Studio or command-line tools
   ```bash
   # If using Android Studio: it installs SDK automatically
   # If command-line only:
   sdkmanager "platform-tools" "platforms;android-34" "build-tools;34.0.0"

   # Accept licenses
   flutter doctor --android-licenses
   ```

3. **Java JDK 17** (required by Gradle)
   ```bash
   # NixOS:
   nix-shell -p jdk17

   # Ubuntu:
   sudo apt install openjdk-17-jdk
   ```

---

## Build Steps

```bash
# 1. Navigate to mobile app directory
cd interfaces/mobile

# 2. Install Flutter dependencies
flutter pub get

# 3. Configure your RedNode server URL
# Edit lib/api/rednode_client.dart line 5:
#   static String baseUrl = 'http://YOUR-REDNODE-IP:8787';
# For Tailscale: 'http://100.x.x.x:8787'

# 4. Build release APK
flutter build apk --release

# The APK is at:
# build/app/outputs/flutter-apk/app-release.apk
# Size: ~20-25 MB

# 5. Install on your phone
# Option A: USB debugging
adb install build/app/outputs/flutter-apk/app-release.apk

# Option B: Transfer APK to phone
# Copy app-release.apk to your phone via USB/cloud/share
# On phone: Settings → Security → Allow unknown sources → install

# Option C: Build APK bundle (for Play Store, if ever needed)
flutter build appbundle --release
# → build/app/outputs/bundle/release/app-release.aab
```

---

## Firebase Setup (for Push Notifications)

Push notifications for approval requests require Firebase Cloud Messaging.

```bash
# 1. Create a Firebase project at https://console.firebase.google.com
# 2. Add an Android app with package name: os.rednode.mobile
# 3. Download google-services.json
# 4. Place it at: interfaces/mobile/android/app/google-services.json
# 5. Rebuild the APK

# See interfaces/mobile/FIREBASE_SETUP.md for detailed instructions
```

**If you DON'T want Firebase** (maximum privacy):
- The app works without it — it polls the API every 5 seconds for approvals
- You just won't get push notifications
- Remove the firebase dependencies from pubspec.yaml if you want a clean build

---

## Testing on Emulator

```bash
# Start Android emulator
flutter emulators --launch Pixel_7_API_34

# Run in debug mode (hot reload)
flutter run

# The emulator uses 10.0.2.2 to reach host localhost
# So the default baseUrl 'http://10.0.2.2:8787' should work
```

---

## Connecting to Your RedNode Server

The mobile app needs to reach your RedNode CNS. Options:

1. **Same WiFi (LAN)** — set baseUrl to `http://10.0.50.10:8787` (your VLAN 50 IP)
   - Only works when on your home WiFi
   - Your phone must be on VLAN 10 (trusted)

2. **Tailscale VPN** (recommended) — set baseUrl to `http://100.x.x.x:8787`
   - Works from anywhere
   - Install Tailscale on RedNode server + phone
   - Zero port forwarding needed

3. **WireGuard VPN** — the app has built-in WireGuard support
   - Configure in app Settings → WireGuard
   - Auto-connects when app opens

---

## Signing the APK (for distribution)

```bash
# Generate a keystore (once)
keytool -genkey -v -keystore rednode-release.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias rednode -storepass YOUR_PASSWORD

# Create android/key.properties:
cat > android/key.properties << EOF
storePassword=YOUR_PASSWORD
keyPassword=YOUR_PASSWORD
keyAlias=rednode
storeFile=$(pwd)/rednode-release.jks
EOF

# Build signed APK
flutter build apk --release
# The APK is now signed with your key
```

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `flutter doctor` shows issues | Install missing components per its suggestions |
| Gradle build fails | Ensure JDK 17 (not 21), check `android/app/build.gradle` |
| Can't connect to RedNode | Check firewall rules — VLAN 10 must reach VLAN 50:8787 |
| Biometric not working on emulator | Emulator doesn't have biometrics — test on real device |
| APK too large | `flutter build apk --release --split-per-abi` → separate APKs per architecture |
