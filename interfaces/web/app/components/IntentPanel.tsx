'use client';
import { useState } from 'react';
import { sendIntent } from '../../lib/api';
export default function IntentPanel() {
  const [intent, setIntent] = useState('');
  const [result, setResult] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const send = async () => {
    if (!intent.trim()) return;
    setLoading(true);
    try { const r = await sendIntent(intent); setResult(r); }
    catch(e:any){ setResult({ok:false, error: e.message }) }
    finally { setLoading(false); }
  };
  return (
    <div>
      <h3 style={{marginTop:0}}>Intent → Plan → Execute</h3>
      <div style={{display:'flex', gap:8}}>
        <input value={intent} onChange={e=>setIntent(e.target.value)} onKeyDown={e=>e.key==='Enter'&&send()}
          placeholder='e.g. harden ssh and show docker status'
          style={{flex:1, padding:10, background:'#0b0f14', color:'#e6edf3', border:'1px solid #263042', borderRadius:8}} />
        <button onClick={send} disabled={loading}
          style={{background: loading ? '#30363d' : '#1f6feb', color:'#fff', border:0, padding:'8px 14px', borderRadius:8, cursor:'pointer'}}>
          {loading ? '…' : 'Send'}
        </button>
      </div>
      {result && (
        <div style={{marginTop:12, fontSize:13}}>
          <b>Plan:</b>
          <ol>
            {(result.plan || []).map((p:any,i:number)=><li key={i}>{p.tool} – <span style={{opacity:.7}}>{p.agent}</span> – <span style={{color: p.risk==='high' ? '#f85149' : p.risk==='medium' ? '#d29922' : '#3fb950'}}>{p.risk}</span></li>)}
          </ol>
          <b>Results:</b>
          <pre style={{background:'#0b0f14', padding:10, borderRadius:8, fontSize:12, whiteSpace:'pre-wrap', maxHeight:260, overflow:'auto'}}>
            {JSON.stringify(result.results, null, 2)}
          </pre>
        </div>
      )}
      <div style={{marginTop:8, fontSize:12, opacity:.7}}>
        CNS: Rust / Axum – NATS – Postgres / Qdrant / Kuzu – Ollama qwen2.5:14b
      </div>
    </div>
  );
}
