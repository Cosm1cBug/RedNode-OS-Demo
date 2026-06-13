'use client';
import { useState } from 'react';
import IntentPanel from './components/IntentPanel';
import SentiencePanel from './components/SentiencePanel';
import ApprovalQueue from './components/ApprovalQueue';
import MemoryBrowser from './components/MemoryBrowser';
import SecurityFeed from './components/SecurityFeed';
import AuditLog from './components/AuditLog';
import AgentStatus from './components/AgentStatus';
import EventStream from './components/EventStream';

const tabs = [
  { id: 'intent', label: 'Intent', component: IntentPanel },
  { id: 'sentience', label: 'Sentience', component: SentiencePanel },
  { id: 'approvals', label: 'Approvals', component: ApprovalQueue },
  { id: 'memory', label: 'Memory', component: MemoryBrowser },
  { id: 'security', label: 'Security', component: SecurityFeed },
  { id: 'audit', label: 'Audit', component: AuditLog },
  { id: 'agents', label: 'Agents', component: AgentStatus },
  { id: 'events', label: 'Live Events', component: EventStream },
] as const;

export default function Page() {
  const [tab, setTab] = useState<typeof tabs[number]['id']>('intent');
  const Active = tabs.find(t => t.id === tab)?.component || IntentPanel;
  return (
    <div style={{minHeight:'100vh', background:'#0b0f14', color:'#e6edf3'}}>
      <header style={{padding:'14px 24px', borderBottom:'1px solid #1c232d', background:'#0f141b', display:'flex', justifyContent:'space-between', alignItems:'center'}}>
        <div><b style={{fontSize:18}}>🧠 RedNode-OS</b> <span style={{opacity:.7, marginLeft:12}}>Personal Autonomous Operating System</span></div>
        <div style={{fontSize:13, opacity:.7}}>CNS: Rust • NATS • Postgres/Qdrant/Kuzu • Ollama</div>
      </header>
      <nav style={{padding:'0 24px', borderBottom:'1px solid #1c232d', display:'flex', gap:18, background:'#0f141b'}}>
        {tabs.map(t => (
          <button key={t.id} onClick={()=>setTab(t.id)}
            style={{
              background:'transparent', border:0, color: tab===t.id ? '#e6edf3' : '#8b949e',
              padding:'12px 2px', cursor:'pointer',
              borderBottom: tab===t.id ? '2px solid #1f6feb' : '2px solid transparent',
              fontWeight: tab===t.id ? 600 : 400
            }}>{t.label}</button>
        ))}
      </nav>
      <main style={{padding:24, maxWidth:1200, margin:'0 auto'}}>
        <div style={{background:'#121820', border:'1px solid #1c232d', borderRadius:12, padding:18, minHeight:420}}>
          <Active />
        </div>
        <footer style={{marginTop:16, fontSize:12, opacity:.6, textAlign:'center'}}>
          RedNode is not an AI. RedNode is a society of specialized agents. • Security is the foundation • Intelligence is the operating layer
        </footer>
      </main>
    </div>
  );
}
