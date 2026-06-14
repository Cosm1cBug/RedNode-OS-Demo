'use client';
import { useEffect, useState } from 'react';
import { getNasHealth, getNasUsage } from '../../lib/api';

export default function StoragePanel() {
  const [health, setHealth] = useState<any>(null);
  const [usage, setUsage] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const [h, u] = await Promise.all([getNasHealth(), getNasUsage()]);
      setHealth(h);
      setUsage(u);
    } catch {}
    setLoading(false);
  };

  useEffect(() => { load(); const t = setInterval(load, 30000); return () => clearInterval(t); }, []);

  const healthOutput = health?.results?.[0]?.result?.output || health?.results?.[0]?.result?.result?.output || '';
  const usageOutput = usage?.results?.[0]?.result?.output || usage?.results?.[0]?.result?.result?.output || '';

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>💾 Storage — TrueNAS</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8, cursor:'pointer'}}>Refresh</button>
      </div>

      {loading ? (
        <div style={{opacity:.6}}>Loading TrueNAS status...</div>
      ) : (
        <div>
          {healthOutput ? (
            <div style={{marginBottom:16}}>
              <h4 style={{margin:'0 0 8px', fontSize:14, opacity:.8}}>Pool Health</h4>
              <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:12, margin:0}}>
                {healthOutput}
              </pre>
            </div>
          ) : null}

          {usageOutput ? (
            <div style={{marginBottom:16}}>
              <h4 style={{margin:'0 0 8px', fontSize:14, opacity:.8}}>Storage Usage</h4>
              <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:12, margin:0}}>
                {usageOutput}
              </pre>
            </div>
          ) : null}

          {!healthOutput && !usageOutput && (
            <div style={{opacity:.6}}>
              TrueNAS not reachable. Set TRUENAS_URL and TRUENAS_API_KEY environment variables for the storage-agent.
              <br/><br/>
              <code style={{fontSize:11}}>TRUENAS_URL=https://10.0.50.3 TRUENAS_API_KEY=your-api-key</code>
            </div>
          )}

          <div style={{fontSize:12, opacity:.6, marginTop:8}}>
            Storage Agent: pool health · SMART · snapshots · shares · RedNode brain backup · Auto-refresh: 30s
          </div>
        </div>
      )}
    </div>
  );
}
