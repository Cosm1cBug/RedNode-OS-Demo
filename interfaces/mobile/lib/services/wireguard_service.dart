import 'dart:io';
import 'package:flutter/services.dart';

/// RedNode WireGuard Auto-Tunnel
/// 
/// Privacy-first remote access – all traffic to your RedNode goes via
/// WireGuard / Tailscale – zero open inbound ports on RedNode.
/// 
/// Strategy:
/// 1. Try native WireGuard tunnel via MethodChannel (android: VpnService + wireguard-go)
///    – requires `com.wireguard.android:tunnel:1.0` – see android/ build.gradle
/// 2. Fallback: launch WireGuard / Tailscale app via Intent
/// 3. Fallback: instruct user to enable VPN manually
///
/// For production RedNode Mobile:
/// - Embed wireguard-go via gomobile
/// - Store private key in flutter_secure_storage / Android Keystore
/// - Auto-connect on app start
/// - Kill-switch: block API calls if tunnel down

class WireGuardService {
  static const _channel = MethodChannel('os.rednode/wireguard');
  static bool _useNative = false;

  /// Try to establish a WireGuard tunnel to your RedNode
  /// config: standard WireGuard .conf content
  /// Returns true if tunnel is up
  static Future<bool> connect({String? config}) async {
    // 1. Try native tunnel – via MethodChannel → Android VpnService + wireguard-go
    if (_useNative) {
      try {
        final ok = await _channel.invokeMethod<bool>('wg_connect', {'config': config});
        if (ok == true) return true;
      } catch (_) {}
    }

    // 2. Try launching WireGuard / Tailscale app
    // WireGuard Android – tunnel toggle intent:
    // adb shell am start -a com.wireguard.android.action.SET_TUNNEL_UP -e tunnel "rednode"
    if (Platform.isAndroid) {
      try {
        // Try Tailscale first – most users will use Tailscale for RedNode
        // const tailscaleIntent = 'com.tailscale.ipn.ui.MainActivity';
        // Then WireGuard
        await _channel.invokeMethod('wg_launch_app');
        // User will need to toggle manually – return false to show "VPN required" banner
        return false;
      } catch (_) {}
    }

    // 3. Fallback – instruct user
    return false;
  }

  static Future<void> disconnect() async {
    try { await _channel.invokeMethod('wg_disconnect'); } catch (_) {}
  }

  static Future<bool> isConnected() async {
    try {
      final s = await _channel.invokeMethod<String>('wg_status');
      return s == 'up';
    } catch (_) {
      return false;
    }
  }

  /// Check if we're on a trusted network – Tailscale IP 100.x / CGNAT
  /// RedNode CNS should ONLY be reachable via VPN – never public internet
  static Future<bool> isOnTrustedNetwork(String nodeUrl) async {
    final uri = Uri.tryParse(nodeUrl);
    if (uri == null) return false;
    final host = uri.host;
    // Tailscale
    if (host.startsWith('100.')) return true;
    // Local LAN – 192.168/10/172.16
    if (host.startsWith('192.168.') || host.startsWith('10.') || RegExp(r'^172\.(1[6-9]|2\d|3[01])\.').hasMatch(host)) return true;
    // localhost / emulator
    if (host == 'localhost' || host == '10.0.2.2' || host == '127.0.0.1') return true;
    // .tailnet.ts.net
    if (host.endsWith('.ts.net')) return true;
    return false;
  }
}
