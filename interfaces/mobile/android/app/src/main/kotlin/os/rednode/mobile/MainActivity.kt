package os.rednode.mobile

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import android.content.Intent
import android.net.VpnService

class MainActivity: FlutterActivity() {
    private val WIREGUARD_CHANNEL = "os.rednode/wireguard"
    
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        
        // WireGuard VPN Tunnel – MethodChannel bridge
        // Production: integrate wireguard-go / boringtun via gomobile
        // For now: launch WireGuard / Tailscale app, or use VpnService stub
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, WIREGUARD_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "wg_status" -> {
                    // TODO: query VpnService – check if tun0 is up and peer is RedNode
                    // For now: check if Tailscale / WireGuard app is running
                    result.success("down")
                }
                "wg_connect" -> {
                    // Try to launch WireGuard / Tailscale
                    try {
                        // Try Tailscale first
                        var intent = packageManager.getLaunchIntentForPackage("com.tailscale.ipn")
                        if (intent == null) {
                            // Fall back to WireGuard
                            intent = packageManager.getLaunchIntentForPackage("com.wireguard.android")
                        }
                        if (intent != null) {
                            startActivity(intent)
                            result.success(false) // user needs to toggle manually
                        } else {
                            result.error("NO_VPN_APP", "Install Tailscale or WireGuard from Play Store / F-Droid", null)
                        }
                    } catch (e: Exception) {
                        result.error("VPN_ERROR", e.message, null)
                    }
                }
                "wg_disconnect" -> {
                    result.success(true)
                }
                "wg_launch_app" -> {
                    try {
                        var intent = packageManager.getLaunchIntentForPackage("com.tailscale.ipn")
                            ?: packageManager.getLaunchIntentForPackage("com.wireguard.android")
                        if (intent != null) {
                            startActivity(intent)
                            result.success(true)
                        } else {
                            result.error("NO_VPN_APP", "Install Tailscale", null)
                        }
                    } catch (e: Exception) {
                        result.error("ERR", e.message, null)
                    }
                }
                else -> result.notImplemented()
            }
        }
    }
    
    // VpnService permission – for future native WireGuard tunnel
    // override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
    //   if (requestCode == 1001 && resultCode == RESULT_OK) {
    //     // VpnService.prepare() granted – start wireguard-go tunnel
    //   }
    // }
}
