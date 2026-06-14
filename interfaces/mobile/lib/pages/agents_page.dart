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
  bool loading = true;

  Future<void> load() async {
    try {
      final a = await widget.client.getAgents();
      final s = await widget.client.getSentience();
      if (mounted) setState(() { agents = a; sentience = s; loading = false; });
    } catch (_) {
      if (mounted) setState(() => loading = false);
    }
  }

  @override void initState() { super.initState(); load(); }

  IconData _agentIcon(String name) {
    if (name.contains('system')) return Icons.settings;
    if (name.contains('security')) return Icons.security;
    if (name.contains('coding')) return Icons.code;
    if (name.contains('research')) return Icons.search;
    if (name.contains('automation')) return Icons.auto_awesome;
    if (name.contains('network')) return Icons.language;
    if (name.contains('infra')) return Icons.dns;
    if (name.contains('storage')) return Icons.storage;
    if (name.contains('surveillance')) return Icons.videocam;
    if (name.contains('comms')) return Icons.email;
    return Icons.smart_toy;
  }

  Color _statusColor(dynamic agent) {
    final alive = agent['alive'] ?? (agent['status'] == 'online');
    final stale = agent['status'] == 'stale';
    if (alive == true) return Colors.green;
    if (stale) return Colors.amber;
    return Colors.red;
  }

  @override Widget build(BuildContext context) {
    final drives = sentience?['model']?['drives'];
    final resources = sentience?['model']?['resources'];
    final goalsExecuted = sentience?['model']?['goals_executed'] ?? 0;
    final uptime = sentience?['model']?['uptime_secs'] ?? 0;

    return RefreshIndicator(
      onRefresh: load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Header
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Agent Society', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
              Text('${agents.length} agents', style: const TextStyle(opacity: 0.7, fontSize: 13)),
            ],
          ),
          const SizedBox(height: 12),

          // Agent cards
          if (loading) const Center(child: CircularProgressIndicator()),
          if (!loading && agents.isEmpty)
            const Text('No agents connected. Start agents with: pnpm agents',
              style: TextStyle(opacity: 0.6)),
          ...agents.map((a) => Card(
            color: const Color(0xFF121820),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(10),
              side: BorderSide(color: _statusColor(a).withOpacity(0.5)),
            ),
            child: ListTile(
              leading: Icon(_agentIcon(a['name'] ?? ''), color: _statusColor(a)),
              title: Text(a['name'] ?? 'unknown'),
              subtitle: Text(
                'Status: ${a['status']} · Tasks: ${a['tasks_completed'] ?? 0}',
                style: const TextStyle(fontSize: 12),
              ),
              trailing: Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: _statusColor(a).withOpacity(0.2),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Text(
                  a['alive'] == true ? 'online' : a['status'] ?? '?',
                  style: TextStyle(fontSize: 11, color: _statusColor(a)),
                ),
              ),
            ),
          )),

          const SizedBox(height: 20),

          // Sentience Drives
          if (drives != null) ...[
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('Sentience – Homeostatic Drives',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
                Text('Uptime: ${(uptime / 60).toStringAsFixed(0)}m',
                  style: const TextStyle(fontSize: 12, opacity: 0.6)),
              ],
            ),
            const SizedBox(height: 12),
            ...['security','integrity','knowledge','energy','availability'].map((k) {
              final v = ((drives[k] ?? 0.0) as num).toDouble();
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 4),
                child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text(k, style: const TextStyle(fontSize: 13)),
                      Text('${(v*100).toStringAsFixed(0)}%', style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
                    ],
                  ),
                  const SizedBox(height: 2),
                  LinearProgressIndicator(
                    value: v, minHeight: 8,
                    borderRadius: BorderRadius.circular(4),
                    backgroundColor: Colors.grey.shade900,
                    valueColor: AlwaysStoppedAnimation(
                      v > 0.8 ? Colors.green : v > 0.6 ? Colors.amber : Colors.red),
                  ),
                ]),
              );
            }),
          ],

          const SizedBox(height: 16),

          // Resources
          if (resources != null)
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: const Color(0xFF0D1117),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Resources', style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600)),
                  const SizedBox(height: 6),
                  Text('CPU: ${(resources['cpu_percent'] ?? 0).toStringAsFixed(1)}% · '
                    'RAM: ${resources['mem_used_mb'] ?? 0}/${resources['mem_total_mb'] ?? 0} MB · '
                    'Disk: ${(resources['disk_used_pct'] ?? 0).toStringAsFixed(0)}% · '
                    'Temp: ${(resources['temp_c'] ?? 0).toStringAsFixed(0)}°C',
                    style: const TextStyle(fontSize: 12, fontFamily: 'monospace'),
                  ),
                  const SizedBox(height: 4),
                  Text('Goals executed: $goalsExecuted',
                    style: const TextStyle(fontSize: 12, opacity: 0.7)),
                ],
              ),
            ),

          const SizedBox(height: 20),
          const Text(
            'RedNode is not an AI. RedNode is a society of specialized agents.\n'
            'Security is the foundation. Intelligence is the operating layer.',
            style: TextStyle(fontSize: 11, color: Colors.grey),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}
