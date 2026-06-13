# RedNode Mobile – Firebase / FCM Setup

RedNode uses Firebase Cloud Messaging ONLY for push-notification transport – payload is encrypted end-to-end, Firebase never sees approval contents.

The app works fully offline / without Firebase – approvals are polled every 5s as fallback.

To enable push approvals:

1. **Create Firebase project**
   ```
   # Install flutterfire_cli
   dart pub global activate flutterfire_cli

   cd interfaces/mobile
   flutterfire configure \
     --project=rednode-os \
     --out=lib/firebase_options.dart \
     --android-package-name=os.rednode.mobile \
     --ios-bundle-id=os.rednode.mobile
   ```
   This generates:
   - `android/app/google-services.json`
   - `lib/firebase_options.dart`

2. **Enable Cloud Messaging**
   Firebase Console → Cloud Messaging → Enable

3. **Android – add google-services plugin**
   In `android/build.gradle`:
   ```
   dependencies {
     classpath 'com.google.gms:google-services:4.4.1'
   }
   ```
   In `android/app/build.gradle` – uncomment:
   ```
   id "com.google.gms.google-services"
   ```
   And dependencies:
   ```
   implementation platform('com.google.firebase:firebase-bom:32.8.0')
   implementation 'com.google.firebase:firebase-messaging'
   ```

4. **Register FCM token with RedNode CNS**
   The app automatically calls:
   ```
   POST /api/mobile/register { fcm_token: "..." }
   ```
   – implement in `RedNodeClient.registerFcmToken()` – currently logs token only

   RedNode CNS (Rust) needs Firebase Admin SDK to send pushes:
   - Add `fcm_v1` crate, or call FCM HTTP v1 with service account JSON
   - Store at `/var/lib/rednode/secrets/fcm-service-account.json` – sops encrypted
   - When approval is created → push to registered devices

5. **Build APK**
   ```
   cd interfaces/mobile
   flutter pub get
   flutter build apk --release
   # output: build/app/outputs/flutter-apk/app-release.apk
   ```

   Sideload: `adb install build/app/outputs/flutter-apk/app-release.apk`

**Privacy Note:**
- FCM is transport-only – approval payload is encrypted with a pre-shared key derived from your RedNode node identity (age/X25519)
- You can run fully without Firebase – the app polls every 5s
- For 100% de-googled: use ntfy / UnifiedPush – hook is in `FirebaseMessagingService` – swap easily

---

**Biometric Approval – already wired**
- Uses `local_auth` – Android BiometricPrompt / iOS FaceID
- Every High/Critical approval requires biometric – see `ApprovalsPage.act()`
- Falls back to device PIN – configurable to biometric-only

**WireGuard Auto-Tunnel**
- `lib/services/wireguard_service.dart`
- Tries native tunnel via MethodChannel → VpnService + wireguard-go
- Fallback: launches WireGuard / Tailscale app via Intent
- UI shows tunnel status in Settings – green = connected
- All API calls should check `WireGuardService.isOnTrustedNetwork()` – RedNode CNS rejects non-VPN IPs via firewall

**Security – Mobile**
- API token / node URL / WireGuard private key → `flutter_secure_storage` – Android Keystore / iOS Keychain – hardware-backed
- No logs with secrets
- Certificate pinning – TODO: add `ssl_pinning_plugin` with your RedNode self-signed CA
- Biometric required for approvals
- Screen capture blocked in approval flow – TODO: `FLAG_SECURE`
- All traffic via WireGuard – 0.0.0.0/0 blocked on RedNode firewall

See also: `SECURITY.md` in repo root
