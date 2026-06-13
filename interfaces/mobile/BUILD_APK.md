# RedNode Mobile – Build APK

Privacy-first Android remote for RedNode-OS

## Prerequisites
- Flutter 3.22+
- Android SDK 34, NDK 25.1
- JDK 17

```bash
cd interfaces/mobile
flutter pub get
```

## Firebase – Push Approvals (optional)
FCM is transport-only – payload E2EE – app works fully offline polling every 5s without Firebase.

To enable push:
```bash
dart pub global activate flutterfire_cli
flutterfire configure \
  --project=rednode-os \
  --out=lib/firebase_options.dart \
  --android-package-name=os.rednode.mobile \
  --ios-bundle-id=os.rednode.mobile
```
This overwrites:
- `android/app/google-services.json`
- `lib/firebase_options.dart`

Then uncomment in:
- `android/build.gradle` → `classpath 'com.google.gms:google-services:4.4.1'`
- `android/app/build.gradle` → `id "com.google.gms.google-services"`

And in `lib/main.dart`:
```dart
import 'firebase_options.dart';
await Firebase.initializeApp(options: DefaultFirebaseOptions.currentPlatform);
```

Without Firebase: app polls CNS every 5s – fully functional, 100% private.

## Biometric Approval
- Uses `local_auth` – Android BiometricPrompt / iOS FaceID
- Every High/Critical approval requires biometric
- See: `lib/services/biometric_auth.dart`
- Falls back to device PIN if biometric unavailable (configurable to biometric-only)

## WireGuard Auto-Tunnel
- `lib/services/wireguard_service.dart`
- Tries native VpnService + wireguard-go via MethodChannel
  – see `android/app/src/main/kotlin/os/rednode/mobile/MainActivity.kt`
- Fallback: launches WireGuard / Tailscale app via Intent
- UI shows tunnel status – blocks API calls if not on trusted network
- Trusted networks: 100.x (Tailscale), 192.168/10/172.16 (LAN), localhost
- **RedNode CNS rejects all non-VPN IPs – firewall DROP by default**

For full in-app WireGuard (no external app):
1. Build wireguard-go for Android: `gomobile bind -target=android golang.zx2c4.com/wireguard`
2. Wire the tunnel to VpnService in `MainActivity.kt`
3. Store private key in `flutter_secure_storage` – Android Keystore – hardware-backed

Current build uses Intent fallback – user taps "Connect" → opens Tailscale/WireGuard app – secure, simple, auditable.

## Build

Debug (USB):
```
flutter run
# Select Android device
# App connects to http://10.0.2.2:8787 (emulator → host)
# Set your RedNode Tailscale IP in Settings → Node URL
```

Release APK – sideload:
```
flutter build apk --release
adb install build/app/outputs/flutter-apk/app-release.apk
```

Release AAB – Play Store:
```
flutter build appbundle --release
# build/app/outputs/bundle/release/app-release.aab
```

APK size: ~24 MB (Flutter + arm64 + armv7)

## Security – Mobile
- API token / node URL / WireGuard private key → `flutter_secure_storage`
  – Android Keystore – hardware-backed – AES-256-GCM
- Biometric required for approvals – `local_auth`
- Certificate pinning – TODO: add `ssl_pinning_plugin` with your RedNode self-signed CA
- Screen capture blocked in approval flow – TODO: `FLAG_SECURE`
- Network Security Config – cleartext allowed only for 10.0.2.2 / 192.168 / 100.x – production: use https with self-signed
- No analytics, no crashlytics, no ads – 0 trackers
- FCM payload is E2EE – Firebase sees only a ping, not the approval content
- All CNS traffic via WireGuard – RedNode firewall DROP all non-VPN

## Permissions
- INTERNET
- ACCESS_NETWORK_STATE
- POST_NOTIFICATIONS – approval pushes
- USE_BIOMETRIC / USE_FINGERPRINT
- CAMERA – optional – QR code node onboarding
- FOREGROUND_SERVICE / RECEIVE_BOOT_COMPLETED – persistent VPN notification

See: `android/app/src/main/AndroidManifest.xml`

---

RedNode Mobile – your private, security-first remote into your personal autonomous OS.
The interface is just a window. The intelligence remains inside RedNode.
