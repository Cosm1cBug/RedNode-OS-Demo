'use client';
import { useEvents } from '../../lib/useEvents';
export default function EventStream() {
  const events = useEvents();
  return (
    <div>
      <h3 style={{marginTop:0}}>Live CNS Events</h3>
      <div style={{height:300, overflow:'auto', fontFamily:'ui-monospace,monospace', fontSize:12, background:'#0b0f14', border:'1px solid #30363d', borderRadius:8, padding:8}}>
        {events.length===0 && <div style={{opacity:.6}}>Connecting to ws://localhost:8787/events …</div>}
        {events.map((e,i)=><div key={i} style={{borderBottom:'1px solid #161b22', padding:'2px 0'}}>{e}</div>)}
      </div>
    </div>
  );
}
