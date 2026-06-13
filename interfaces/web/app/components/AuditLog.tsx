'use client';
import { useEffect, useState } from 'react';
import { getAudit } from '../../lib/api';
export default function AuditLog() {
  const [rows, setRows] = useState<any[]>([]);
  const load = async () => { const d = await getAudit(100).catch(()=>({entries:[]})); setRows(d.entries || []); };
  useEffect(() => { load(); const t = setInterval(load, 5000); return ()=>clearInterval(t); }, []);
  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>Audit Log – Hash-chained, tamper-evident</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8}}>Refresh</button>
      </div>
      <div style={{maxHeight:480, overflow:'auto', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0b0f14', border:'1px solid #30363d', borderRadius:8}}>
        <table style={{width:'100%', borderCollapse:'collapse'}}>
          <thead style={{position:'sticky', top:0, background:'#161b22'}}><tr>
            <th style={{textAlign:'left', padding:'8px'}}>ID</th>
            <th style={{textAlign:'left', padding:'8px'}}>Time</th>
            <th style={{textAlign:'left', padding:'8px'}}>Actor</th>
            <th style={{textAlign:'left', padding:'8px'}}>Tool</th>
            <th style={{textAlign:'left', padding:'8px'}}>Risk</th>
            <th style={{textAlign:'left', padding:'8px'}}>Result</th>
            <th style={{textAlign:'left', padding:'8px'}}>Hash</th>
          </tr></thead>
          <tbody>
          {rows.map((r:any)=>(
            <tr key={r.id} style={{borderTop:'1px solid #21262d'}}>
              <td style={{padding:'6px 8px'}}>{r.id}</td>
              <td style={{padding:'6px 8px', whiteSpace:'nowrap'}}>{new Date(r.ts).toLocaleTimeString()}</td>
              <td style={{padding:'6px 8px'}}>{r.actor}</td>
              <td style={{padding:'6px 8px'}}>{r.tool || r.action}</td>
              <td style={{padding:'6px 8px'}}>{r.risk || '-'}</td>
              <td style={{padding:'6px 8px', maxWidth:300, overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap'}}>{(r.result||'').slice(0,80)}</td>
              <td style={{padding:'6px 8px', opacity:.6}}>{r.hash ? r.hash.slice(0,10)+'…' : '-'}</td>
            </tr>
          ))}
          </tbody>
        </table>
        {rows.length===0 && <div style={{padding:12, opacity:.7}}>No audit entries yet – run an intent to generate audit trail.</div>}
      </div>
      <div style={{marginTop:8, fontSize:12, opacity:.7}}>Every tool execution is logged with SHA-256 hash chaining – prev_hash → hash – tamper-evident.</div>
    </div>
  );
}
