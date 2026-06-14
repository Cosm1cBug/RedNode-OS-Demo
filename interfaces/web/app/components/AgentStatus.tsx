'use client';
import { useEffect, useState } from 'react';
import { getAgents } from '../../lib/api';

interface Agent {
  name: string;
  status: string;
  alive: boolean;
  last_heartbeat: string;
  tasks_completed: number;
}

export default function AgentStatus() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    try {
      const data = await getAgents();
      setAgents(data.agents || []);
    } catch {}
    setLoading(false);
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, []);

  const agentIcon = (name: string) => {
    if (name.includes('system')) return '🔧';
    if (name.includes('security')) return '🛡️';
    if (name.includes('coding')) return '💻';
    if (name.includes('research')) return '🔬';
    if (name.includes('automation')) return '⚙️';
    if (name.includes('network')) return '🌐';
    if (name.includes('infra')) return '🏗️';
    if (name.includes('storage')) return '💾';
    if (name.includes('surveillance')) return '📹';
    if (name.includes('comms')) return '📧';
    return '🤖';
  };

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>Agent Society</h3>
        <button onClick={refresh} style={{background:'#1f6feb', border:0, color:'#fff', padding:'4px 12px', borderRadius:6, cursor:'pointer', fontSize:12}}>
          Refresh
        </button>
      </div>
      {loading ? (
        <div style={{opacity:0.6}}>Loading agents...</div>
      ) : agents.length === 0 ? (
        <div style={{opacity:0.6}}>No agents connected. Start agents with: pnpm agents</div>
      ) : (
        <div style={{display:'grid', gridTemplateColumns:'repeat(auto-fill, minmax(280px, 1fr))', gap:10}}>
          {agents.map(a => {
            const isAlive = a.alive !== false && a.status === 'online';
            const isStale = a.status === 'stale';
            const borderColor = isAlive ? '#238636' : isStale ? '#d29922' : '#da3633';
            const bgColor = isAlive ? 'rgba(35,134,54,0.1)' : isStale ? 'rgba(210,153,34,0.1)' : 'rgba(218,54,51,0.1)';

            return (
              <div key={a.name} style={{
                background: bgColor,
                border: `1px solid ${borderColor}`,
                borderRadius: 8,
                padding: 12,
              }}>
                <div style={{display:'flex', justifyContent:'space-between', alignItems:'center'}}>
                  <span style={{fontWeight:600, fontSize:14}}>
                    {agentIcon(a.name)} {a.name}
                  </span>
                  <span style={{
                    fontSize:11, padding:'2px 8px', borderRadius:12,
                    background: isAlive ? '#238636' : isStale ? '#d29922' : '#da3633',
                    color: '#fff',
                  }}>
                    {a.status}
                  </span>
                </div>
                <div style={{fontSize:12, opacity:0.7, marginTop:6}}>
                  Tasks: {a.tasks_completed || 0}
                  {a.last_heartbeat && (
                    <span style={{marginLeft:12}}>
                      Last seen: {new Date(a.last_heartbeat).toLocaleTimeString()}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
      <div style={{fontSize:11, opacity:0.5, marginTop:12}}>
        {agents.length} agents tracked · Heartbeat interval: 15s · Stale threshold: 45s
      </div>
    </div>
  );
}
