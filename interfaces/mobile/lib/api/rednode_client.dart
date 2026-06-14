import 'dart:convert';
import 'package:http/http.dart' as http;

class RedNodeClient {
  static String baseUrl = 'http://10.0.2.2:8787'; // Android emulator → host
  static String apiToken = ''; // Set via settings page

  Map<String, String> get _h {
    final headers = {'Content-Type': 'application/json'};
    if (apiToken.isNotEmpty) {
      headers['Authorization'] = 'Bearer $apiToken';
    }
    return headers;
  }

  // ─── Core ───

  Future<Map<String, dynamic>> sendIntent(String intent, {String sessionId = 'mobile'}) async {
    final r = await http.post(Uri.parse('$baseUrl/intent'),
      headers: _h, body: jsonEncode({'intent': intent, 'session_id': sessionId}));
    return jsonDecode(r.body);
  }

  Future<Map<String, dynamic>> getHealth() async {
    final r = await http.get(Uri.parse('$baseUrl/health'), headers: _h);
    return jsonDecode(r.body);
  }

  // ─── Approvals ───

  Future<List<dynamic>> getApprovals() async {
    final r = await http.get(Uri.parse('$baseUrl/approvals'), headers: _h);
    final j = jsonDecode(r.body);
    return (j['approvals'] ?? []) as List;
  }

  Future<bool> approve(String id, bool approved) async {
    final r = await http.post(Uri.parse('$baseUrl/approvals/$id/approve'),
      headers: _h, body: jsonEncode({'approved': approved}));
    return jsonDecode(r.body)['ok'] == true;
  }

  // ─── Security ───

  Future<List<dynamic>> getSecurityEvents() async {
    final r = await http.get(Uri.parse('$baseUrl/security/events'), headers: _h);
    final j = jsonDecode(r.body);
    return (j['events'] ?? []) as List;
  }

  Future<bool> ackSecurityEvent(String id) async {
    final r = await http.post(Uri.parse('$baseUrl/security/events/$id/ack'), headers: _h);
    return jsonDecode(r.body)['ok'] == true;
  }

  // ─── Memory ───

  Future<List<dynamic>> queryMemory(String q) async {
    final r = await http.get(
      Uri.parse('$baseUrl/memory/query?q=${Uri.encodeComponent(q)}'), headers: _h);
    final j = jsonDecode(r.body);
    return (j['results'] ?? []) as List;
  }

  // ─── Audit ───

  Future<List<dynamic>> getAudit() async {
    final r = await http.get(Uri.parse('$baseUrl/audit?limit=100'), headers: _h);
    final j = jsonDecode(r.body);
    return (j['entries'] ?? []) as List;
  }

  // ─── Agents ───

  Future<List<dynamic>> getAgents() async {
    final r = await http.get(Uri.parse('$baseUrl/agents/status'), headers: _h);
    final j = jsonDecode(r.body);
    return (j['agents'] ?? []) as List;
  }

  // ─── Sentience ───

  Future<Map<String, dynamic>> getSentience() async {
    final r = await http.get(Uri.parse('$baseUrl/sentience'), headers: _h);
    return jsonDecode(r.body);
  }

  // ─── Infrastructure shortcuts (via intents) ───

  Future<Map<String, dynamic>> getPiholeStats() =>
    sendIntent('show pihole stats', sessionId: 'mobile');

  Future<Map<String, dynamic>> getNasHealth() =>
    sendIntent('check TrueNAS pool health', sessionId: 'mobile');

  Future<Map<String, dynamic>> getCameraEvents() =>
    sendIntent('show recent camera events', sessionId: 'mobile');

  Future<Map<String, dynamic>> getEmailSummary() =>
    sendIntent('summarize my recent emails', sessionId: 'mobile');

  Future<Map<String, dynamic>> runWorkflow(String name) =>
    sendIntent('run workflow $name', sessionId: 'mobile');
}
