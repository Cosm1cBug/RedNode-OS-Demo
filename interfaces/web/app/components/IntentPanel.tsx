'use client';
import { useState } from 'react';
import { sendIntent } from '../../lib/api';

const SUGGESTIONS = [
  "check system health",
  "show Pi-hole DNS stats",
  "who was at the front door today?",
  "show my tasks",
  "run workflow morning",
  "check TrueNAS pool health",
  "show camera events",
  "scan for CVEs",
  "search the web for NixOS tips",
  "show security events",
];

export default function IntentPanel() {
  const [input, setInput] = useState('');
  const [result, setResult] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [history, setHistory] = useState<{intent:string, ts:string}[]>([]);

  const submit = async (text?: string) => {
    const intent = (text || input).trim();
    if (!intent) return;
    setLoading(true);
    setResult(null);
    try {
      const r = await sendIntent(intent);
      setResult(r);
      setHistory(prev => [{intent, ts: new Date().toLocaleTimeString()}, ...prev].slice(0, 10));
    } catch (e: any) {
      setResult({ ok: false, error: e.message });
    }
    setLoading(false);
    if (!text) setInput('');
  };

  const riskColor = (r: string) => r === 'low' ? 'var(--green)' : r === 'medium' ? 'var(--amber)' : r === 'high' ? 'var(--red)' : '#f85149';
  const statusIcon = (s: string) => s === 'executed' ? '✅' : s === 'needs_approval' ? '⏳' : s === 'denied' ? '🚫' : '❌';

  return (
    <div>
      <div style={{display:'flex', alignItems:'center', gap:10, marginBottom:16}}>
        <h3 style={{margin:0, fontSize:16, fontWeight:600}}>🎯 Express an Intention</h3>
        <span style={{fontSize:12, color:'var(--text-muted)'}}>Natural language → LLM plan → sandboxed execution → audit</span>
      </div>

      {/* Input */}
      <div style={{display:'flex', gap:8, marginBottom:12}}>
        <input
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && submit()}
          placeholder="What do you want RedNode to do?"
          style={{
            flex:1, padding:'10px 14px', fontSize:14,
            background:'var(--bg-primary)', border:'1px solid var(--border)',
            borderRadius:'var(--radius-md)', color:'var(--text-primary)',
            outline:'none', transition:'border var(--transition)',
          }}
          onFocus={e => e.currentTarget.style.borderColor = 'var(--accent)'}
          onBlur={e => e.currentTarget.style.borderColor = 'var(--border)'}
        />
        <button
          onClick={() => submit()}
          disabled={loading}
          style={{
            padding:'10px 20px', fontSize:14, fontWeight:600,
            background: loading ? 'var(--border)' : 'var(--accent)',
            color:'#fff', border:'none', borderRadius:'var(--radius-md)',
          }}>
          {loading ? '⏳ Planning...' : '→ Send'}
        </button>
      </div>

      {/* Quick suggestions */}
      <div style={{display:'flex', flexWrap:'wrap', gap:6, marginBottom:16}}>
        {SUGGESTIONS.slice(0, 5).map(s => (
          <button key={s} onClick={() => { setInput(s); submit(s); }}
            style={{
              padding:'4px 10px', fontSize:11, background:'var(--bg-elevated)',
              border:'1px solid var(--border)', borderRadius:20, color:'var(--text-secondary)',
            }}>
            {s}
          </button>
        ))}
      </div>

      {/* Results */}
      {result && (
        <div className="animate-in">
          {/* Plan */}
          {result.plan?.length > 0 && (
            <div style={{marginBottom:14}}>
              <div style={{fontSize:13, fontWeight:600, marginBottom:8, color:'var(--text-secondary)'}}>
                📋 Plan ({result.plan.length} steps)
              </div>
              <div style={{display:'flex', flexDirection:'column', gap:4}}>
                {result.plan.map((p: any, i: number) => (
                  <div key={i} style={{
                    display:'flex', alignItems:'center', gap:8, padding:'6px 10px',
                    background:'var(--bg-primary)', borderRadius:'var(--radius-sm)',
                    border:'1px solid var(--border)', fontSize:13,
                  }}>
                    <span style={{color:'var(--text-muted)', fontSize:11, minWidth:20}}>{i+1}.</span>
                    <code style={{fontFamily:'var(--font-mono)', fontWeight:600}}>{p.tool}</code>
                    <span style={{color:'var(--text-muted)', fontSize:12}}>→ {p.agent}</span>
                    <span style={{
                      marginLeft:'auto', padding:'1px 8px', borderRadius:10, fontSize:10, fontWeight:600,
                      background: `color-mix(in srgb, ${riskColor(p.risk)} 15%, transparent)`,
                      color: riskColor(p.risk),
                      border: `1px solid color-mix(in srgb, ${riskColor(p.risk)} 30%, transparent)`,
                    }}>{p.risk}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Results */}
          {result.results?.length > 0 && (
            <div>
              <div style={{fontSize:13, fontWeight:600, marginBottom:8, color:'var(--text-secondary)'}}>
                📊 Results
              </div>
              {result.results.map((r: any, i: number) => (
                <div key={i} className="animate-in" style={{
                  background:'var(--bg-primary)', borderRadius:'var(--radius-md)',
                  border:'1px solid var(--border)', padding:12, marginBottom:8,
                }}>
                  <div style={{display:'flex', alignItems:'center', gap:8, marginBottom:6}}>
                    <span>{statusIcon(r.status)}</span>
                    <code style={{fontFamily:'var(--font-mono)', fontWeight:600, fontSize:13}}>{r.tool}</code>
                    <span style={{fontSize:12, color:'var(--text-muted)'}}>— {r.status}</span>
                  </div>
                  {(r.result?.output || r.result?.result?.output) && (
                    <pre style={{
                      margin:0, padding:10, fontSize:12, lineHeight:1.5,
                      background:'var(--bg-secondary)', borderRadius:'var(--radius-sm)',
                      overflowX:'auto', maxHeight:300, whiteSpace:'pre-wrap',
                      color:'var(--text-secondary)', fontFamily:'var(--font-mono)',
                    }}>
                      {String(r.result?.output || r.result?.result?.output).substring(0, 2000)}
                    </pre>
                  )}
                </div>
              ))}
            </div>
          )}

          {result.error && (
            <div style={{padding:12, background:'var(--red-dim)', border:'1px solid var(--red)', borderRadius:'var(--radius-md)', color:'var(--red)'}}>
              ❌ {result.error}
            </div>
          )}
        </div>
      )}

      {/* History */}
      {history.length > 0 && !loading && !result && (
        <div style={{marginTop:12}}>
          <div style={{fontSize:12, color:'var(--text-muted)', marginBottom:6}}>Recent</div>
          {history.map((h, i) => (
            <button key={i} onClick={() => { setInput(h.intent); submit(h.intent); }}
              style={{
                display:'block', width:'100%', textAlign:'left', padding:'6px 10px',
                background:'transparent', border:'none', color:'var(--text-secondary)',
                fontSize:12, borderBottom:'1px solid var(--border)',
              }}>
              <span style={{color:'var(--text-muted)', marginRight:8}}>{h.ts}</span>
              {h.intent}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
