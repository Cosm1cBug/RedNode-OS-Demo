import 'package:flutter/material.dart';
import '../api/rednode_client.dart';

class MemoryPage extends StatefulWidget {
  final RedNodeClient client;
  const MemoryPage({super.key, required this.client});
  @override State<MemoryPage> createState() => _MemoryPageState();
}

class _MemoryPageState extends State<MemoryPage> {
  final ctrl = TextEditingController(text: 'RedNode agents');
  List results = [];
  bool loading = false;
  Future<void> search() async {
    setState(() => loading = true);
    try { final r = await widget.client.queryMemory(ctrl.text); setState(() => results = r); }
    finally { setState(() => loading = false); }
  }
  @override void initState() { super.initState(); search(); }
  @override Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        const Text('Memory Browser – RAG / Knowledge Graph', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
        const SizedBox(height: 12),
        Row(children: [
          Expanded(child: TextField(controller: ctrl, decoration: const InputDecoration(border: OutlineInputBorder(), hintText: 'Search Long-Term / Vector / Kuzu…'), onSubmitted: (_)=>search())),
          const SizedBox(width:8),
          FilledButton(onPressed: loading ? null : search, child: Text(loading ? '…' : 'Search')),
        ]),
        const SizedBox(height: 12),
        Expanded(child: results.isEmpty ? const Center(child: Text('No results – try a query', style: TextStyle(color: Colors.grey))) : ListView.builder(
          itemCount: results.length,
          itemBuilder: (_, i) {
            final r = results[i];
            return Card(color: const Color(0xFF121820), child: ListTile(
              title: Text(r['content'] ?? '', maxLines: 3, overflow: TextOverflow.ellipsis),
              subtitle: Text('${r['source'] ?? 'memory'} • score ${r['score'] ?? '—'}', style: const TextStyle(fontSize: 11)),
            ));
          },
        )),
        const SizedBox(height: 8),
        const Text('Backends: PostgreSQL + Qdrant (768d) + Kuzu\nTiers: Long-Term • Working • Episodic • Security',
          style: TextStyle(fontSize: 11, color: Colors.grey)),
      ]),
    );
  }
}
