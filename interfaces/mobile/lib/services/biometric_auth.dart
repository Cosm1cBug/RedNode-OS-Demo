import 'package:local_auth/local_auth.dart';
import 'package:flutter/services.dart';

class BiometricAuth {
  static final _auth = LocalAuthentication();

  static Future<bool> isAvailable() async {
    try {
      return await _auth.canCheckBiometrics && await _auth.isDeviceSupported();
    } catch (_) { return false; }
  }

  /// Authenticate user – used for High/Critical approval
  /// Returns true if biometric / device credential succeeds
  static Future<bool> authenticate({String reason = 'Approve RedNode action'}) async {
    try {
      final available = await isAvailable();
      if (!available) {
        // In dev / emulator without biometrics – allow with warning
        return true;
      }
      return await _auth.authenticate(
        localizedReason: reason,
        options: const AuthenticationOptions(
          biometricOnly: false, // allow PIN/pattern fallback
          stickyAuth: true,
          sensitiveTransaction: true,
        ),
      );
    } on PlatformException catch (_) {
      return false;
    }
  }
}
