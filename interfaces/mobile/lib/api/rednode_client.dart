import 'dart:convert';
import 'package:http/http.dart' as http;

class RedNodeClient {
  static String baseUrl = 'http://10.0.2.2:8787'; // Android emulator → host
  // Set to your Tailscale IP in production: https://rednode.tailnet:8787

  Map<String, String> get _h => {'Content-Type': 'application/json'};

  Future<Map<String, dynamic>> sendIntent(String intent, {String sessionId = 'mobile'}) async {
    final r = await http.post(Uri.parse('$baseUrl/intent'),
      headers: _h, body: jsonEncode({'intent': intent, 'session_id': sessionId}));
    return jsonDecode(r.body);
  }

  Future<List<dynamic>> getApprovals() async {
    final r = await http.get(Uri.parse('$baseUrl/approvals'));
    final j = jsonDecode(r.body);
    return (j['approvals'] ?? []) as List;
  }

  Future<bool> approve(String id, bool approved) async {
    final r = await http.post(Uri.parse('$baseUrl/approvals/$id/approve'),
      headers: _h, body: jsonEncode({'approved': approved}));
    return jsonDecode(r.body)['ok'] == true;
  }

  Future<List<dynamic>> getSecurityEvents() async {
    final r = await http.get(Uri.parse('$baseUrl/security/events'));
    final j = jsonDecode(r.body);
    return (j['events'] ?? []) as List;
  }

  Future<bool> ackSecurityEvent(String id) async {
    final r = await http.post(Uri.parse('$baseUrl/security/events/$id/ack'));
    return jsonDecode(r.body)['ok'] == true;
  }

  Future<List<dynamic>> queryMemory(String q) async {
    final r = await http.get(Uri.parse('$baseUrl/memory/query?q=${Uri.encodeComponent(q)}'));
    final j = jsonDecode(r.body);
    return (j['results'] ?? []) as List;
  }

  Future<List<dynamic>> getAudit() async {
    final r = await http.get(Uri.parse('$baseUrl/audit?limit=100'));
    final j = jsonDecode(r.body);
    return (j['entries'] ?? []) as List;
  }

  Future<List<dynamic>> getAgents() async {
    final r = await http.get(Uri.parse('$baseUrl/agents/status'));
    final j = jsonDecode(r.body);
    return (j['agents'] ?? []) as List;
  }

  Future<Map<String, dynamic>> getSentience() async {
    final r = await http.get(Uri.parse('$baseUrl/sentience'));
    return jsonDecode(r.body);
  }
}
