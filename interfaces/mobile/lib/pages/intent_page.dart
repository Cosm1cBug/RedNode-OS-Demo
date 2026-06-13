import 'package:flutter/material.dart';
import '../api/rednode_client.dart';

class IntentPage extends StatefulWidget {
  final RedNodeClient client;
  const IntentPage({super.key, required this.client});
  @override State<IntentPage> createState() => _IntentPageState();
}

class _IntentPageState extends State<IntentPage> {
  final ctrl = TextEditingController();
  Map<String, dynamic>? result;
  bool loading = false;

  Future<void> send() async {
    if (ctrl.text.trim().isEmpty) return;
    setState(() => loading = true);
    try {
      final r = await widget.client.sendIntent(ctrl.text);
      setState(() => result = r);
    } catch (e) {
      setState(() => result = {'ok': false, 'error': e.toString()});
    } finally {
      setState(() => loading = false);
    }
  }

  @override Widget build(BuildContext context) {
    return ListView(padding: const EdgeInsets.all(16), children: [
      const Text('Intent → Plan → Execute', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
      const SizedBox(height: 12),
      TextField(
        controller: ctrl,
        decoration: const InputDecoration(
          labelText: 'Express an intention',
          hintText: 'harden ssh and show docker status',
          border: OutlineInputBorder(),
        ),
        minLines: 1, maxLines: 3,
        onSubmitted: (_) => send(),
      ),
      const SizedBox(height: 12),
      FilledButton.icon(
        onPressed: loading ? null : send,
        icon: loading ? const SizedBox(width:16,height:16,child:CircularProgressIndicator(strokeWidth:2, color: Colors.white)) : const Icon(Icons.send),
        label: Text(loading ? 'Routing via CNS…' : 'Send Intent'),
      ),
      if (result != null) ...[
        const SizedBox(height: 16),
        const Text('Plan', style: TextStyle(fontWeight: FontWeight.bold)),
        ...(result!['plan'] as List? ?? []).map((p) => ListTile(
          dense: true,
          title: Text(p['tool'] ?? ''),
          subtitle: Text('${p['agent'] ?? ''} • risk: ${p['risk'] ?? ''}'),
        )),
        const SizedBox(height: 8),
        const Text('Results', style: TextStyle(fontWeight: FontWeight.bold)),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(color: const Color(0xFF121820), borderRadius: BorderRadius.circular(8)),
          child: Text(
            const JsonEncoder.withIndent('  ').convert(result!['results'] ?? {}),
            style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
          ),
        ),
      ],
      const SizedBox(height: 24),
      const Text('RedNode is not an AI. RedNode is a society of specialized agents.',
        style: TextStyle(fontSize: 12, color: Colors.grey), textAlign: TextAlign.center),
    ]);
  }
}
