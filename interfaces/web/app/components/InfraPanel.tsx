'use client';
import { useEffect, useState } from 'react';
import { getPiholeStats } from '../../lib/api';

export default function InfraPanel() {
  const [data, setData] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const r = await getPiholeStats();
      setData(r);
    } catch {}
    setLoading(false);
  };

  useEffect(() => { load(); const t = setInterval(load, 15000); return () => clearInterval(t); }, []);

  const output = data?.results?.[0]?.result?.output || data?.results?.[0]?.result?.result?.output || '';
  const stats = data?.results?.[0]?.result?.stats || data?.results?.[0]?.result?.result?.stats || null;

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>🏗️ Infrastructure — Pi-hole DNS</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8, cursor:'pointer'}}>Refresh</button>
      </div>

      {loading ? (
        <div style={{opacity:.6}}>Loading Pi-hole stats...</div>
      ) : stats ? (
        <div>
          <div style={{display:'grid', gridTemplateColumns:'repeat(auto-fill, minmax(200px, 1fr))', gap:12, marginBottom:16}}>
            {[
              { label: 'Total Queries', value: stats.queries?.total || stats.dns_queries_today || '—', color: '#1f6feb' },
              { label: 'Blocked', value: stats.queries?.blocked || stats.ads_blocked_today || '—', color: '#f85149' },
              { label: 'Block Rate', value: `${(stats.queries?.percent_blocked || stats.ads_percentage_today || 0).toFixed(1)}%`, color: '#d29922' },
              { label: 'Domains on Blocklist', value: stats.gravity?.domains_being_blocked || stats.domains_being_blocked || '—', color: '#8b949e' },
            ].map((s, i) => (
              <div key={i} style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:14, textAlign:'center'}}>
                <div style={{fontSize:24, fontWeight:700, color:s.color}}>{s.value}</div>
                <div style={{fontSize:12, opacity:.7, marginTop:4}}>{s.label}</div>
              </div>
            ))}
          </div>
          <div style={{fontSize:12, opacity:.6}}>
            DNS server: Pi-hole · Upstream: Quad9 / Unbound · Auto-refresh: 15s
          </div>
        </div>
      ) : output ? (
        <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:12}}>
          {output}
        </pre>
      ) : (
        <div style={{opacity:.6}}>
          Pi-hole not reachable. Set PIHOLE_URL and PIHOLE_PASSWORD environment variables for the infra-agent.
          <br/><br/>
          <code style={{fontSize:11}}>PIHOLE_URL=http://10.0.50.2 PIHOLE_PASSWORD=yourpassword</code>
        </div>
      )}
    </div>
  );
}
