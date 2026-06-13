import 'package:flutter/material.dart';
import '../api/rednode_client.dart';

class AgentsPage extends StatefulWidget {
  final RedNodeClient client;
  const AgentsPage({super.key, required this.client});
  @override State<AgentsPage> createState() => _AgentsPageState();
}

class _AgentsPageState extends State<AgentsPage> {
  List agents = [];
  Map<String, dynamic>? sentience;
  Future<void> load() async {
    try {
      final a = await widget.client.getAgents();
      if (mounted) setState(() => agents = a);
      final s = await widget.client.getSentience();
      if (mounted) setState(() => sentience = s);
    } catch (_) {}
  }
  @override void initState() { super.initState(); load(); }

  @override Widget build(BuildContext context) {
    final drives = sentience?['model']?['drives'];
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        const Text('Agent Society', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
        const SizedBox(height: 8),
        ...agents.map((a) => Card(
          color: const Color(0xFF121820),
          child: ListTile(
            leading: const Icon(Icons.smart_toy, color: Colors.green),
            title: Text(a['name']),
            subtitle: Text('status: ${a['status']} • ${a['last_heartbeat']}'),
          ),
        )),
        const SizedBox(height: 16),
        if (drives != null) ...[
          const Text('Sentience – Homeostatic Drives', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
          const SizedBox(height: 8),
          ...['security','integrity','knowledge','energy','availability'].map((k) {
            final v = ((drives[k] ?? 0.0) as num).toDouble();
            return Padding(
              padding: const EdgeInsets.symmetric(vertical: 4),
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Text('$k  ${(v*100).toStringAsFixed(0)}%'),
                LinearProgressIndicator(value: v, minHeight: 6,
                  backgroundColor: Colors.grey.shade900,
                  valueColor: AlwaysStoppedAnimation(
                    v > 0.8 ? Colors.green : v > 0.6 ? Colors.amber : Colors.red)),
              ]),
            );
          }),
          const SizedBox(height: 8),
          const Text('RedNode is not an AI. RedNode is a society of specialized agents.\nSecurity is the foundation. Intelligence is the operating layer.',
            style: TextStyle(fontSize: 12, color: Colors.grey)),
        ],
      ],
    );
  }
}
