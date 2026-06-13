'use client';
import { useEffect, useState } from 'react';
import { getApprovals, approve } from '../../lib/api';
export default function ApprovalQueue() {
  const [items, setItems] = useState<any[]>([]);
  const load = async () => { const d = await getApprovals().catch(()=>({approvals:[]})); setItems(d.approvals || []); };
  useEffect(() => { load(); const t = setInterval(load, 3000); return ()=>clearInterval(t); }, []);
  const act = async (id:string, ok:boolean) => { await approve(id, ok); load(); };
  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>Approval Queue – {items.length} pending</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8}}>Refresh</button>
      </div>
      {items.length === 0 && <div style={{opacity:.7}}>No pending approvals – all high/critical actions require explicit consent.</div>}
      {items.map((a:any)=>(
        <div key={a.id} style={{border:'1px solid #30363d', borderRadius:10, padding:12, marginBottom:10, background:'#0d1117'}}>
          <div style={{fontSize:12, opacity:.7}}>{new Date(a.ts).toLocaleString()} • <b style={{color:'#f85149'}}>{a.risk.toUpperCase()}</b> • {a.actor}</div>
          <div style={{fontFamily:'ui-monospace,monospace', marginTop:6}}>{a.tool}</div>
          <pre style={{fontSize:12, opacity:.8, margin:'6px 0', whiteSpace:'pre-wrap'}}>{JSON.stringify(a.args, null, 2)}</pre>
          {a.intent && <div style={{fontSize:13, opacity:.8}}>Intent: “{a.intent}”</div>}
          <div style={{marginTop:8, display:'flex', gap:8}}>
            <button onClick={()=>act(a.id, true)} style={{background:'#238636', color:'#fff', border:0, padding:'6px 12px', borderRadius:8, cursor:'pointer'}}>Approve</button>
            <button onClick={()=>act(a.id, false)} style={{background:'#da3633', color:'#fff', border:0, padding:'6px 12px', borderRadius:8, cursor:'pointer'}}>Deny</button>
            <span style={{fontSize:12, opacity:.6, alignSelf:'center'}}>ID: {a.id.slice(0,8)}…</span>
          </div>
        </div>
      ))}
    </div>
  );
}
