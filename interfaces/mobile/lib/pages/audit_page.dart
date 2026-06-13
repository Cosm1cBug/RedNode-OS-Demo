import 'package:flutter/material.dart';
import '../api/rednode_client.dart';

class AuditPage extends StatefulWidget {
  final RedNodeClient client;
  const AuditPage({super.key, required this.client});
  @override State<AuditPage> createState() => _AuditPageState();
}

class _AuditPageState extends State<AuditPage> {
  List rows = [];
  Future<void> load() async {
    final r = await widget.client.getAudit();
    setState(() => rows = r);
  }
  @override void initState() { super.initState(); load(); }
  @override Widget build(BuildContext context) {
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.all(16),
          child: Row(children: [
            const Expanded(child: Text('Audit Log – Hash-chained, tamper-evident', style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold))),
            IconButton(onPressed: load, icon: const Icon(Icons.refresh)),
          ]),
        ),
        Expanded(
          child: ListView.builder(
            itemCount: rows.length,
            itemBuilder: (_, i) {
              final r = rows[i];
              return ListTile(
                dense: true,
                title: Text('${r['tool'] ?? r['action'] ?? ''} • ${r['actor'] ?? ''}', style: const TextStyle(fontFamily: 'monospace', fontSize: 13)),
                subtitle: Text('${r['risk'] ?? ''} • ${(r['result'] ?? '').toString()}', maxLines: 1, overflow: TextOverflow.ellipsis),
                trailing: Text('#${r['id']}', style: const TextStyle(fontSize: 11, color: Colors.grey)),
              );
            },
          ),
        ),
      ],
    );
  }
}
