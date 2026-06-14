'use client';
import { useState } from 'react';
import { runWorkflow, sendIntent } from '../../lib/api';

const BUILT_IN_WORKFLOWS = [
  { name: 'goodnight', icon: '🌙', label: 'Goodnight', description: 'Night mode — strict DNS, camera alerts, snapshot, consolidation' },
  { name: 'morning', icon: '☀️', label: 'Morning Brief', description: 'System health, overnight events, DNS stats, storage status' },
  { name: 'focus', icon: '🎯', label: 'Focus Mode', description: 'Block social media DNS, minimize distractions' },
  { name: 'leaving', icon: '🚪', label: 'Leaving Home', description: 'All cameras active, enable remote access' },
];

export default function WorkflowPanel() {
  const [running, setRunning] = useState<string | null>(null);
  const [results, setResults] = useState<any>(null);
  const [customIntent, setCustomIntent] = useState('');
  const [customResults, setCustomResults] = useState<any>(null);

  const run = async (name: string) => {
    setRunning(name);
    setResults(null);
    try {
      const r = await runWorkflow(name);
      setResults(r);
    } catch (e: any) {
      setResults({ error: e.message });
    }
    setRunning(null);
  };

  const runCustom = async () => {
    if (!customIntent.trim()) return;
    setRunning('custom');
    setCustomResults(null);
    try {
      const r = await sendIntent(customIntent.trim());
      setCustomResults(r);
    } catch (e: any) {
      setCustomResults({ error: e.message });
    }
    setRunning(null);
  };

  return (
    <div>
      <h3 style={{margin:'0 0 12px'}}>⚙️ Workflows & Automation</h3>

      {/* Built-in workflows */}
      <div style={{display:'grid', gridTemplateColumns:'repeat(auto-fill, minmax(250px, 1fr))', gap:10, marginBottom:20}}>
        {BUILT_IN_WORKFLOWS.map(wf => (
          <button
            key={wf.name}
            onClick={() => run(wf.name)}
            disabled={running !== null}
            style={{
              background: '#0d1117',
              border: '1px solid #30363d',
              borderRadius: 10,
              padding: 14,
              cursor: running ? 'wait' : 'pointer',
              textAlign: 'left',
              color: '#e6edf3',
              opacity: running && running !== wf.name ? 0.5 : 1,
              transition: 'border-color 0.2s',
            }}
            onMouseEnter={e => (e.currentTarget.style.borderColor = '#1f6feb')}
            onMouseLeave={e => (e.currentTarget.style.borderColor = '#30363d')}
          >
            <div style={{fontSize:20, marginBottom:6}}>{wf.icon}</div>
            <div style={{fontWeight:600, fontSize:14}}>{wf.label}</div>
            <div style={{fontSize:12, opacity:.7, marginTop:4}}>{wf.description}</div>
            {running === wf.name && (
              <div style={{fontSize:11, color:'#1f6feb', marginTop:6}}>Running...</div>
            )}
          </button>
        ))}
      </div>

      {/* Results from workflow */}
      {results && (
        <div style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:12, marginBottom:16}}>
          <h4 style={{margin:'0 0 8px', fontSize:13}}>Workflow Results</h4>
          {results.results ? (
            results.results.map((r: any, i: number) => (
              <div key={i} style={{borderBottom:'1px solid #1c232d', padding:'6px 0', fontSize:12}}>
                {r.status === 'executed' ? '✅' : r.status === 'needs_approval' ? '⏳' : '❌'}{' '}
                <b>{r.tool}</b>: {r.status}
                {r.result?.output && (
                  <div style={{opacity:.7, marginTop:2, fontFamily:'ui-monospace,monospace', fontSize:11}}>
                    {String(r.result.output).substring(0, 200)}
                  </div>
                )}
              </div>
            ))
          ) : results.error ? (
            <div style={{color:'#f85149'}}>{results.error}</div>
          ) : (
            <pre style={{fontSize:11, margin:0}}>{JSON.stringify(results, null, 2)}</pre>
          )}
        </div>
      )}

      {/* Custom intent */}
      <div style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:14}}>
        <h4 style={{margin:'0 0 8px', fontSize:13}}>Custom Workflow / Intent</h4>
        <div style={{display:'flex', gap:8}}>
          <input
            value={customIntent}
            onChange={e => setCustomIntent(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && runCustom()}
            placeholder="e.g., create a snapshot of documents and check camera events"
            style={{
              flex:1, background:'#0b0f14', border:'1px solid #30363d', borderRadius:6,
              padding:'8px 12px', color:'#e6edf3', fontSize:13,
            }}
          />
          <button
            onClick={runCustom}
            disabled={running !== null}
            style={{background:'#1f6feb', color:'#fff', border:0, padding:'8px 16px', borderRadius:6, cursor:'pointer', fontSize:13}}
          >
            {running === 'custom' ? 'Running...' : 'Execute'}
          </button>
        </div>
        {customResults && (
          <div style={{marginTop:10}}>
            {(customResults.results || []).map((r: any, i: number) => (
              <div key={i} style={{fontSize:12, padding:'4px 0', borderBottom:'1px solid #1c232d'}}>
                {r.status === 'executed' ? '✅' : '❌'} {r.tool}: {r.status}
              </div>
            ))}
          </div>
        )}
      </div>

      <div style={{fontSize:12, opacity:.6, marginTop:12}}>
        Automation Agent: workflows, scheduling, triggers · Built-in: goodnight, morning, focus, leaving
      </div>
    </div>
  );
}
