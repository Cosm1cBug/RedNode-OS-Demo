'use client';
import { useState, useEffect } from 'react';
import IntentPanel from './components/IntentPanel';
import SentiencePanel from './components/SentiencePanel';
import ApprovalQueue from './components/ApprovalQueue';
import MemoryBrowser from './components/MemoryBrowser';
import SecurityFeed from './components/SecurityFeed';
import AuditLog from './components/AuditLog';
import AgentStatus from './components/AgentStatus';
import EventStream from './components/EventStream';
import InfraPanel from './components/InfraPanel';
import StoragePanel from './components/StoragePanel';
import CameraPanel from './components/CameraPanel';
import CommsPanel from './components/CommsPanel';
import WorkflowPanel from './components/WorkflowPanel';

const tabs = [
  { id: 'intent', label: 'Intent', icon: '🎯', component: IntentPanel },
  { id: 'sentience', label: 'Sentience', icon: '🧠', component: SentiencePanel },
  { id: 'workflows', label: 'Workflows', icon: '⚙️', component: WorkflowPanel },
  { id: 'infra', label: 'DNS', icon: '🏗️', component: InfraPanel },
  { id: 'storage', label: 'Storage', icon: '💾', component: StoragePanel },
  { id: 'cameras', label: 'Cameras', icon: '📹', component: CameraPanel },
  { id: 'comms', label: 'Comms', icon: '📧', component: CommsPanel },
  { id: 'security', label: 'Security', icon: '🛡️', component: SecurityFeed },
  { id: 'approvals', label: 'Approvals', icon: '✅', component: ApprovalQueue },
  { id: 'memory', label: 'Memory', icon: '📚', component: MemoryBrowser },
  { id: 'audit', label: 'Audit', icon: '📋', component: AuditLog },
  { id: 'agents', label: 'Agents', icon: '🤖', component: AgentStatus },
  { id: 'events', label: 'Live', icon: '⚡', component: EventStream },
] as const;

export default function Page() {
  const [tab, setTab] = useState<typeof tabs[number]['id']>('intent');
  const [time, setTime] = useState('');
  const Active = tabs.find(t => t.id === tab)?.component || IntentPanel;

  useEffect(() => {
    const update = () => setTime(new Date().toLocaleTimeString());
    update();
    const t = setInterval(update, 1000);
    return () => clearInterval(t);
  }, []);

  return (
    <div style={{minHeight:'100vh', background:'var(--bg-primary)'}}>
      {/* Header */}
      <header style={{
        padding:'12px 24px',
        borderBottom:'1px solid var(--border)',
        background:'var(--bg-secondary)',
        display:'flex',
        justifyContent:'space-between',
        alignItems:'center',
        position:'sticky',
        top:0,
        zIndex:100,
        backdropFilter:'blur(12px)',
      }}>
        <div style={{display:'flex', alignItems:'center', gap:12}}>
          <div style={{
            width:36, height:36, borderRadius:'var(--radius-md)',
            background:'linear-gradient(135deg, var(--accent), var(--purple))',
            display:'flex', alignItems:'center', justifyContent:'center',
            fontSize:18, fontWeight:700,
          }}>R</div>
          <div>
            <div style={{fontSize:15, fontWeight:700, letterSpacing:'-0.02em'}}>RedNode-OS</div>
            <div style={{fontSize:11, color:'var(--text-muted)'}}>Personal Autonomous Operating System</div>
          </div>
        </div>
        <div style={{display:'flex', alignItems:'center', gap:16}}>
          <div style={{fontSize:12, color:'var(--text-muted)', fontFamily:'var(--font-mono)'}}>{time}</div>
          <div style={{
            padding:'3px 10px', borderRadius:12, fontSize:11, fontWeight:600,
            background:'var(--green-dim)', color:'var(--green)', border:'1px solid rgba(46,160,67,0.3)',
          }}>
            <span style={{display:'inline-block', width:6, height:6, borderRadius:'50%', background:'var(--green)', marginRight:6, verticalAlign:'middle'}}></span>
            Online
          </div>
        </div>
      </header>

      {/* Navigation */}
      <nav style={{
        padding:'0 24px',
        borderBottom:'1px solid var(--border)',
        background:'var(--bg-secondary)',
        display:'flex',
        gap:2,
        overflowX:'auto',
        whiteSpace:'nowrap',
        msOverflowStyle:'none',
        scrollbarWidth:'none',
      }}>
        {tabs.map(t => (
          <button key={t.id} onClick={()=>setTab(t.id)}
            style={{
              background: tab === t.id ? 'var(--bg-elevated)' : 'transparent',
              border: 'none',
              borderBottom: tab === t.id ? '2px solid var(--accent)' : '2px solid transparent',
              color: tab === t.id ? 'var(--text-primary)' : 'var(--text-secondary)',
              padding:'10px 14px',
              fontSize:13,
              fontWeight: tab === t.id ? 600 : 400,
              borderRadius: 'var(--radius-sm) var(--radius-sm) 0 0',
              display:'flex',
              alignItems:'center',
              gap:6,
              flexShrink:0,
              transition:'all var(--transition)',
            }}>
            <span style={{fontSize:14}}>{t.icon}</span>
            {t.label}
          </button>
        ))}
      </nav>

      {/* Main Content */}
      <main style={{padding:'20px 24px', maxWidth:1400, margin:'0 auto'}}>
        <div className="animate-in" key={tab} style={{
          background:'var(--bg-card)',
          border:'1px solid var(--border)',
          borderRadius:'var(--radius-lg)',
          padding:20,
          minHeight:460,
          boxShadow:'var(--shadow)',
        }}>
          <Active />
        </div>
        <footer style={{marginTop:14, fontSize:11, color:'var(--text-muted)', textAlign:'center', letterSpacing:'0.02em'}}>
          14 Agents · 105 Tools · Rust CNS · Local LLM · Zero Cloud · {time}
        </footer>
      </main>
    </div>
  );
}
