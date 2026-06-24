'use client';
import { useEffect, useState } from 'react';

interface Drives { security: number; integrity: number; knowledge: number; energy: number; availability: number }
interface SelfModel {
  node_id: string;
  boot_ts: string;
  agents: {name: string; status: string; tasks_completed: number}[];
  resources: { cpu_percent: number; mem_used_mb: number; mem_total_mb: number; disk_used_gb: number; disk_total_gb: number; load_avg: number; temp_c: number };
  drives: Drives;
  goals: {id: string; drive: string; description: string; priority: number; created_at: string}[];
  last_introspection: string;
}

const CNS = process.env.NEXT_PUBLIC_CNS || 'http://localhost:8787';

export default function SentiencePanel() {
  const [model, setModel] = useState<SelfModel | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      const r = await fetch(`${CNS}/sentience`, { cache: 'no-store' });
      const j = await r.json();
      if (j.ok && j.model) setModel(j.model);
      else setError('Sentience Engine offline – set REDNODE_SENTIENCE=on');
    } catch (e: any) {
      setError(e.message);
    }
  };

  useEffect(() => { load(); const t = setInterval(load, 2000); return () => clearInterval(t); }, []);

  if (error) return <div style={{color:'#f85149'}}>Sentience offline: {error}<br/><span style={{opacity:.7, fontSize:12}}>Start rednode-core with REDNODE_SENTIENCE=on</span></div>;
  if (!model) return <div>Loading self-model…</div>;

  const drives = model.drives;
  const driveList: [keyof Drives, string, string][] = [
    ['security', 'Security', 'Threat detection, CVE monitoring, self-healing'],
    ['integrity', 'Integrity', 'System health, services up, agent heartbeats'],
    ['knowledge', 'Knowledge', 'RAG coverage, memory freshness, graph completeness'],
    ['energy', 'Energy', 'Power / battery – thermal headroom'],
    ['availability', 'Availability', 'Can serve intentions?'],
  ];

  const barColor = (v: number) => v > 0.8 ? '#3fb950' : v > 0.6 ? '#d29922' : '#f85149';
  const memPct = model.resources.mem_total_mb ? Math.round(model.resources.mem_used_mb / model.resources.mem_total_mb * 100) : 0;
  const diskPct = model.resources.disk_total_gb ? Math.round(model.resources.disk_used_gb / model.resources.disk_total_gb * 100) : 0;

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'baseline', marginBottom:12}}>
        <h3 style={{margin:0}}>Sentience Engine – Self-Aware Loop</h3>
        <span style={{fontSize:12, opacity:.7}}>Node: {model.node_id} • uptime: {Math.floor((Date.now() - new Date(model.boot_ts).getTime())/1000/60)} min • {new Date(model.last_introspection).toLocaleTimeString()}</span>
      </div>

      <div style={{display:'grid', gridTemplateColumns:'1fr 1fr', gap:16}}>
        {/* Drives */}
        <div style={{background:'#0b0f14', border:'1px solid #1c232d', borderRadius:10, padding:14}}>
          <b>Homeostatic Drives</b>
          <div style={{marginTop:10, display:'grid', gap:10}}>
            {driveList.map(([key, label, help]) => {
              const v = drives[key] ?? 0;
              return (
                <div key={key}>
                  <div style={{display:'flex', justifyContent:'space-between', fontSize:13}}>
                    <span>{label} <span style={{opacity:.6, fontSize:11}}>– {help}</span></span>
                    <span style={{fontVariantNumeric:'tabular-nums', color: barColor(v)}}>{(v*100).toFixed(0)}%</span>
                  </div>
                  <div style={{height:8, background:'#161b22', borderRadius:6, marginTop:4, overflow:'hidden'}}>
                    <div style={{
                      width: `${v*100}%`,
                      height: '100%',
                      background: barColor(v),
                      transition: 'width 0.5s'
                    }}/>
                  </div>
                </div>
              );
            })}
          </div>
          <div style={{marginTop:10, fontSize:12, opacity:.7}}>
            Drives {'<'} 0.8 generate autonomous goals – Goal-driven execution – not just reactive
          </div>
        </div>

        {/* Resources */}
        <div style={{background:'#0b0f14', border:'1px solid #1c232d', borderRadius:10, padding:14}}>
          <b>System Resources – Live</b>
          <div style={{marginTop:10, fontSize:13, lineHeight:1.9, fontFamily:'ui-monospace,monospace'}}>
            CPU: {model.resources.cpu_percent.toFixed(1)}%<br/>
            MEM: {model.resources.mem_used_mb} / {model.resources.mem_total_mb} MB – {memPct}%<br/>
            DISK: {model.resources.disk_used_gb} / {model.resources.disk_total_gb} GB – {diskPct}%<br/>
            Load: {model.resources.load_avg.toFixed(2)}<br/>
            Temp: {model.resources.temp_c.toFixed(0)}°C
          </div>
          <div style={{marginTop:10, fontSize:12, opacity:.7}}>
            6 agents online – {model.agents.reduce((s,a)=>s+a.tasks_completed,0)} tasks completed total
          </div>
        </div>

        {/* Goals */}
        <div style={{background:'#0b0f14', border:'1px solid #1c232d', borderRadius:10, padding:14, gridColumn:'1 / -1'}}>
          <b>Autonomous Goals – {model.goals.length}</b>
          {model.goals.length === 0 ? (
            <div style={{opacity:.7, marginTop:8, fontSize:13}}>No active autonomous goals – all drives nominal – system is homeostatic.</div>
          ) : (
            <div style={{marginTop:8, display:'grid', gap:8}}>
              {model.goals.slice().reverse().map(g => (
                <div key={g.id} style={{display:'flex', gap:12, fontSize:13, alignItems:'baseline'}}>
                  <span style={{
                    fontSize:11, padding:'2px 8px', borderRadius:20,
                    background: g.drive === 'security' ? '#3d1115' : g.drive === 'integrity' ? '#1c2b1c' : '#1c1f2b',
                    color: g.drive === 'security' ? '#f85149' : '#8b949e',
                    textTransform:'uppercase', letterSpacing:'.04em'
                  }}>{g.drive}</span>
                  <span style={{flex:1}}>{g.description}</span>
                  <span style={{opacity:.6, fontSize:11}}>prio {(g.priority*100).toFixed(0)}% • {new Date(g.created_at).toLocaleTimeString()}</span>
                </div>
              ))}
            </div>
          )}
          <div style={{marginTop:10, fontSize:12, opacity:.7}}>
            Goal Generator runs every 10s – Introspection loop 1 Hz – Memory consolidation every 5 min (“dream cycle”)
          </div>
        </div>

        {/* Agent Heartbeats */}
        <div style={{background:'#0b0f14', border:'1px solid #1c232d', borderRadius:10, padding:14, gridColumn:'1 / -1'}}>
          <b>Agent Society – Heartbeats</b>
          <div style={{display:'grid', gridTemplateColumns:'repeat(auto-fit, minmax(160px,1fr))', gap:8, marginTop:8, fontSize:13}}>
            {model.agents.map(a => (
              <div key={a.name} style={{opacity:.9}}>
                ● <span style={{color:'#3fb950'}}>{a.name}</span><br/>
                <span style={{opacity:.7, fontSize:11}}>{a.tasks_completed} tasks • {a.status}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div style={{marginTop:14, fontSize:12, opacity:.7, textAlign:'center'}}>
        RedNode is self-aware: self-model • homeostatic drives • goal-driven autonomy • continuous introspection • memory consolidation<br/>
        <b>The computer becomes the intelligence.</b> – Privacy-first • Local-first • Security is the foundation
      </div>
    </div>
  );
}
