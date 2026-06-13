'use client';
import { useEffect, useState } from 'react';
import { getAgents } from '../../lib/api';
export default function AgentStatus() {
  const [agents, setAgents] = useState<any[]>([]);
  useEffect(()=>{ getAgents().then(d=>setAgents(d.agents||[])).catch(()=>{}); const t=setInterval(()=>getAgents().then(d=>setAgents(d.agents||[])).catch(()=>{}), 5000); return ()=>clearInterval(t); },[]);
  return (
    <div>
      <h3 style={{marginTop:0}}>Agent Society – 6 specialized agents</h3>
      <div style={{display:'grid', gridTemplateColumns:'repeat(auto-fit, minmax(220px,1fr))', gap:10}}>
        {agents.map((a:any)=>(
          <div key={a.name} style={{border:'1px solid #30363d', borderRadius:10, padding:12, background:'#0d1117'}}>
            <div style={{fontWeight:600}}>{a.name}</div>
            <div style={{fontSize:12, opacity:.7, marginTop:4}}>● <span style={{color:'#3fb950'}}>online</span> • {a.last_heartbeat}</div>
          </div>
        ))}
      </div>
      <div style={{marginTop:10, fontSize:13, opacity:.7}}>
        System • Security (Smart Security Mode – eBPF/Falco) • Coding • Research (Qdrant/Kuzu) • Automation • Network (Zero-Trust)
      </div>
    </div>
  );
}
