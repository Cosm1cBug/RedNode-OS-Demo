'use client';
import { useState } from 'react';
import { getMemory } from '../../lib/api';
export default function MemoryBrowser() {
  const [q, setQ] = useState('RedNode agents');
  const [results, setResults] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const search = async () => {
    setLoading(true);
    try { const d = await getMemory(q); setResults(d.results || []); } finally { setLoading(false); }
  };
  return (
    <div>
      <h3 style={{marginTop:0}}>Memory Browser – RAG / Knowledge Graph</h3>
      <div style={{display:'flex', gap:8, marginBottom:12}}>
        <input value={q} onChange={e=>setQ(e.target.value)} onKeyDown={e=>e.key==='Enter'&&search()}
          placeholder="Search long-term / vector / knowledge graph…"
          style={{flex:1, padding:10, background:'#0b0f14', color:'#e6edf3', border:'1px solid #263042', borderRadius:8}} />
        <button onClick={search} disabled={loading}
          style={{background:'#1f6feb', color:'#fff', border:0, padding:'8px 14px', borderRadius:8}}>{loading?'…':'Search'}</button>
      </div>
      <div style={{display:'grid', gap:8}}>
        {results.map((r:any, i:number)=>(
          <div key={i} style={{border:'1px solid #30363d', borderRadius:8, padding:10, background:'#0d1117'}}>
            <div style={{fontSize:12, opacity:.7}}>{r.source} • score {r.score ?? '—'}</div>
            <div style={{marginTop:4}}>{r.content}</div>
          </div>
        ))}
        {results.length===0 && !loading && <div style={{opacity:.6}}>Enter a query to search Long-Term Memory, Qdrant vector store, and Kuzu knowledge graph.</div>}
      </div>
      <div style={{marginTop:16, fontSize:13, opacity:.7}}>
        <b>Memory Tiers:</b> Long-Term • Working • Episodic • Security<br/>
        Backends: PostgreSQL + Qdrant (768d) + Kuzu
      </div>
    </div>
  );
}
