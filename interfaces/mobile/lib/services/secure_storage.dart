import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

class SecureStore {
  static const _secure = FlutterSecureStorage(
    aOptions: AndroidOptions(encryptedSharedPreferences: true),
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock),
  );

  // API token / node endpoint – sensitive
  static Future<void> setApiToken(String token) => 
    _secure.write(key: 'rednode_api_token', value: token);
  static Future<String?> getApiToken() => 
    _secure.read(key: 'rednode_api_token');

  static Future<void> setNodeUrl(String url) => 
    _secure.write(key: 'rednode_node_url', value: url);
  static Future<String?> getNodeUrl() => 
    _secure.read(key: 'rednode_node_url');

  // WireGuard private key – NEVER in plaintext prefs
  static Future<void> setWgPrivateKey(String key) => 
    _secure.write(key: 'rednode_wg_private', value: key);
  static Future<String?> getWgPrivateKey() => 
    _secure.read(key: 'rednode_wg_private');

  // Non-sensitive prefs – shared_preferences
  static Future<void> setLastNodeName(String name) async {
    final p = await SharedPreferences.getInstance();
    await p.setString('last_node_name', name);
  }
  static Future<String?> getLastNodeName() async {
    final p = await SharedPreferences.getInstance();
    return p.getString('last_node_name');
  }
}
