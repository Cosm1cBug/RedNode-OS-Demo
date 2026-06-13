'use client';
import { useEffect, useState } from 'react';
import { getSecurityEvents, ackSecurityEvent } from '../../lib/api';
export default function SecurityFeed() {
  const [events, setEvents] = useState<any[]>([]);
  const load = async () => { const d = await getSecurityEvents().catch(()=>({events:[]})); setEvents(d.events || []); };
  useEffect(() => { load(); const t = setInterval(load, 5000); return ()=>clearInterval(t); }, []);
  const ack = async (id:string) => { await ackSecurityEvent(id); load(); };
  const color = (s:string) => s==='CRITICAL' ? '#f85149' : s==='HIGH' ? '#d29922' : s==='MEDIUM' ? '#d29922' : '#3fb950';
  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>Security Feed – Smart Security Mode</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8}}>Refresh</button>
      </div>
      {events.length===0 && <div style={{opacity:.7}}>No security events – Security Agent monitoring via eBPF/Falco – 24/7</div>}
      {events.map((e:any)=>(
        <div key={e.id} style={{borderLeft:`4px solid ${color(e.severity)}`, background:'#0d1117', border:'1px solid #30363d', borderLeftWidth:4, borderRadius:8, padding:10, marginBottom:8, opacity: e.acknowledged ? .6 : 1}}>
          <div style={{fontSize:12, opacity:.8}}>
            {new Date(e.ts).toLocaleString()} • <b style={{color: color(e.severity)}}>{e.severity}</b> • {e.source}
            {e.acknowledged && ' • acknowledged'}
          </div>
          <div style={{marginTop:4}}>{e.summary}</div>
          {e.raw && <details style={{marginTop:6, fontSize:12}}><summary style={{cursor:'pointer', opacity:.7}}>raw</summary><pre style={{whiteSpace:'pre-wrap', opacity:.7}}>{JSON.stringify(e.raw, null, 2)}</pre></details>}
          {!e.acknowledged && <button onClick={()=>ack(e.id)} style={{marginTop:6, background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'4px 8px', borderRadius:6, fontSize:12}}>Acknowledge</button>}
        </div>
      ))}
      <div style={{marginTop:12, fontSize:12, opacity:.7}}>
        Sources: CVE auto-checker (6h), Falco eBPF bridge, YARA, lynis – Auto-patcher with snapshot rollback enabled
      </div>
    </div>
  );
}
