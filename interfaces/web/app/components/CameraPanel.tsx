'use client';
import { useEffect, useState } from 'react';
import { getCameraStatus, getCameraEvents } from '../../lib/api';

export default function CameraPanel() {
  const [status, setStatus] = useState<any>(null);
  const [events, setEvents] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    try {
      const [s, e] = await Promise.all([getCameraStatus(), getCameraEvents()]);
      setStatus(s);
      setEvents(e);
    } catch {}
    setLoading(false);
  };

  useEffect(() => { load(); const t = setInterval(load, 10000); return () => clearInterval(t); }, []);

  const statusOutput = status?.results?.[0]?.result?.output || status?.results?.[0]?.result?.result?.output || '';
  const eventsOutput = events?.results?.[0]?.result?.output || events?.results?.[0]?.result?.result?.output || '';

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>📹 Surveillance — Frigate NVR</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8, cursor:'pointer'}}>Refresh</button>
      </div>

      {loading ? (
        <div style={{opacity:.6}}>Loading camera status...</div>
      ) : (
        <div>
          {statusOutput ? (
            <div style={{marginBottom:16}}>
              <h4 style={{margin:'0 0 8px', fontSize:14, opacity:.8}}>Camera Status</h4>
              <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:12, margin:0}}>
                {statusOutput}
              </pre>
            </div>
          ) : null}

          {eventsOutput ? (
            <div style={{marginBottom:16}}>
              <h4 style={{margin:'0 0 8px', fontSize:14, opacity:.8}}>Recent Detections</h4>
              {eventsOutput.split('\n').filter((l: string) => l.trim()).map((line: string, i: number) => {
                const isPerson = line.toLowerCase().includes('person');
                const borderColor = isPerson ? '#d29922' : '#30363d';
                return (
                  <div key={i} style={{
                    borderLeft: `3px solid ${borderColor}`,
                    background: '#0d1117',
                    border: `1px solid ${borderColor}`,
                    borderLeftWidth: 3,
                    borderRadius: 6,
                    padding: 8,
                    marginBottom: 6,
                    fontSize: 12,
                    fontFamily: 'ui-monospace, monospace',
                  }}>
                    {line}
                  </div>
                );
              })}
            </div>
          ) : null}

          {!statusOutput && !eventsOutput && (
            <div style={{opacity:.6}}>
              Frigate NVR not reachable. Start Frigate via docker compose and set FRIGATE_URL for the surveillance-agent.
              <br/><br/>
              <code style={{fontSize:11}}>docker compose up -d frigate</code>
            </div>
          )}

          <div style={{fontSize:12, opacity:.6, marginTop:8}}>
            Surveillance Agent: MQTT event bridge · AI detection (person/car/animal) · Anomaly detection · Auto-refresh: 10s
          </div>
        </div>
      )}
    </div>
  );
}
