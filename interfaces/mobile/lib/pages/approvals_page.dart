import 'dart:convert';
import 'package:flutter/material.dart';
import '../api/rednode_client.dart';
import '../services/biometric_auth.dart';

class ApprovalsPage extends StatefulWidget {
  final RedNodeClient client;
  const ApprovalsPage({super.key, required this.client});
  @override State<ApprovalsPage> createState() => _ApprovalsPageState();
}

class _ApprovalsPageState extends State<ApprovalsPage> {
  List approvals = [];
  bool loading = false;
  
  Future<void> load() async {
    setState(() => loading = true);
    try { 
      final a = await widget.client.getApprovals(); 
      if (mounted) setState(() => approvals = a); 
    } finally { 
      if (mounted) setState(() => loading = false); 
    }
  }
  
  @override void initState() { super.initState(); load(); }

  Future<void> act(String id, String tool, bool approveAction) async {
    // Step 1: Confirm intent
    final confirmed = await showDialog<bool>(context: context, builder: (c) => AlertDialog(
      title: Text(approveAction ? 'Approve action?' : 'Deny action?'),
      content: Text('Tool: $tool\n\n${approveAction ? 'This will execute with elevated privileges on your RedNode.' : 'This will deny execution and log the decision.'}\n\nBiometric authentication required.'),
      actions: [
        TextButton(onPressed: ()=>Navigator.pop(c,false), child: const Text('Cancel')),
        FilledButton(onPressed: ()=>Navigator.pop(c,true), child: Text(approveAction ? 'Continue' : 'Deny')),
      ],
    )) ?? false;
    if (!confirmed) return;

    // Step 2: Biometric authentication – REQUIRED for High/Critical
    if (approveAction) {
      final authed = await BiometricAuth.authenticate(
        reason: 'Approve RedNode tool: $tool'
      );
      if (!authed) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Biometric authentication failed – approval cancelled'), backgroundColor: Colors.red),
          );
        }
        return;
      }
    }

    // Step 3: Send approval to RedNode CNS
    try {
      final ok = await widget.client.approve(id, approveAction);
      if (ok) {
        load();
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(content: Text(approveAction ? '✓ Approved and sent to RedNode' : '✗ Denied')),
          );
        }
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Approval failed: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  @override Widget build(BuildContext context) {
    return RefreshIndicator(
      onRefresh: load,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Row(children: [
            Expanded(child: Text('Approval Queue – ${approvals.length} pending',
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold))),
            IconButton(onPressed: load, icon: const Icon(Icons.refresh)),
          ]),
          const SizedBox(height: 4),
          const Text('High/Critical tool executions require explicit biometric consent – Zero Trust',
            style: TextStyle(fontSize: 12, color: Colors.grey)),
          const SizedBox(height: 12),
          if (loading) const LinearProgressIndicator(),
          if (approvals.isEmpty && !loading)
            const Card(child: Padding(padding: EdgeInsets.all(16),
              child: Text('No pending approvals – all High/Critical actions require explicit consent.\n\nSecurity is the foundation.'))),
          ...approvals.map((a) => Card(
            color: const Color(0xFF121820),
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
                Wrap(spacing: 8, crossAxisAlignment: WrapCrossAlignment.center, children: [
                  Chip(
                    label: Text((a['risk'] ?? 'high').toString().toUpperCase(),
                      style: const TextStyle(fontSize: 11, color: Colors.white)),
                    backgroundColor: (a['risk'] == 'critical') ? Colors.red.shade800 : Colors.orange.shade800,
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                  Text(a['tool'] ?? '',
                    style: const TextStyle(fontFamily: 'monospace', fontWeight: FontWeight.bold)),
                ]),
                if (a['intent'] != null) Padding(
                  padding: const EdgeInsets.only(top: 6),
                  child: Text('Intent: "${a['intent']}"',
                    style: const TextStyle(fontStyle: FontStyle.italic, color: Colors.grey))),
                const SizedBox(height: 6),
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: Colors.black.withOpacity(0.3),
                    borderRadius: BorderRadius.circular(6)),
                  child: Text(
                    const JsonEncoder.withIndent('  ').convert(a['args'] ?? {}),
                    style: const TextStyle(fontFamily: 'monospace', fontSize: 11, color: Colors.grey),
                  ),
                ),
                const SizedBox(height: 10),
                Row(children: [
                  FilledButton.icon(
                    onPressed: () => act(a['id'], a['tool'] ?? '', true),
                    icon: const Icon(Icons.fingerprint, size: 18),
                    label: const Text('Approve'),
                    style: FilledButton.styleFrom(backgroundColor: Colors.green.shade700),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton.icon(
                    onPressed: () => act(a['id'], a['tool'] ?? '', false),
                    icon: const Icon(Icons.close, size: 18),
                    label: const Text('Deny')),
                  const Spacer(),
                  Text((a['id'] ?? '').toString().substring(0,8),
                    style: const TextStyle(fontSize: 11, color: Colors.grey)),
                ])
              ]),
            ),
          )),
        ],
      ),
    );
  }
}
