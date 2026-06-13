import 'package:flutter/material.dart';
import '../api/rednode_client.dart';

class SecurityFeedPage extends StatefulWidget {
  final RedNodeClient client;
  const SecurityFeedPage({super.key, required this.client});
  @override State<SecurityFeedPage> createState() => _SecurityFeedPageState();
}

class _SecurityFeedPageState extends State<SecurityFeedPage> {
  List events = [];
  Future<void> load() async {
    final e = await widget.client.getSecurityEvents();
    if (mounted) setState(() => events = e);
  }
  @override void initState() { super.initState(); load(); }
  
  Color colorFor(String s) {
    switch(s) {
      case 'CRITICAL': return Colors.red;
      case 'HIGH': return Colors.orange;
      case 'MEDIUM': return Colors.amber;
      default: return Colors.green;
    }
  }

  @override Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const Text('Security Feed – Smart Security Mode', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 4),
          const Text('eBPF + Falco • CVE auto-patcher • YARA • Self-healing – 24/7', style: TextStyle(color: Colors.grey, fontSize: 12)),
          const SizedBox(height: 12),
          if (events.isEmpty) const Card(child: Padding(padding: EdgeInsets.all(16), child: Text('No security events – Security Agent monitoring – 0 incidents'))),
          ...events.map((e) => Card(
            color: const Color(0xFF121820),
            child: ListTile(
              leading: Icon(Icons.shield, color: colorFor(e['severity'] ?? '')),
              title: Text(e['summary'] ?? '', style: const TextStyle(fontSize: 14)),
              subtitle: Text('${e['source']} • ${e['ts']?.toString().substring(0,19) ?? ''} • ${(e['acknowledged'] == true) ? 'acknowledged' : 'new'}', style: const TextStyle(fontSize: 11)),
              trailing: (e['acknowledged'] != true) ? TextButton(
                onPressed: () async { await widget.client.ackSecurityEvent(e['id']); load(); },
                child: const Text('ACK')) : null,
            ),
          )),
        ],
      ),
    );
  }
}
