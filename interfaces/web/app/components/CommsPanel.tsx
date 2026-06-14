'use client';
import { useEffect, useState } from 'react';
import { getEmailSummary, getNotificationDigest, sendIntent } from '../../lib/api';

export default function CommsPanel() {
  const [digest, setDigest] = useState<string>('');
  const [emailSummary, setEmailSummary] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [calendarText, setCalendarText] = useState<string>('');

  const loadDigest = async () => {
    try {
      const r = await getNotificationDigest();
      const output = r?.results?.[0]?.result?.output || r?.results?.[0]?.result?.result?.output || '';
      setDigest(output);
    } catch {}
  };

  const loadEmails = async () => {
    try {
      const r = await getEmailSummary();
      const output = r?.results?.[0]?.result?.output || r?.results?.[0]?.result?.result?.output || '';
      setEmailSummary(output);
    } catch {}
  };

  const loadCalendar = async () => {
    try {
      const r = await sendIntent('show calendar events for the next 7 days', 'dashboard');
      const output = r?.results?.[0]?.result?.output || r?.results?.[0]?.result?.result?.output || '';
      setCalendarText(output);
    } catch {}
  };

  const load = async () => {
    setLoading(true);
    await Promise.all([loadDigest(), loadEmails(), loadCalendar()]);
    setLoading(false);
  };

  useEffect(() => { load(); const t = setInterval(load, 60000); return () => clearInterval(t); }, []);

  return (
    <div>
      <div style={{display:'flex', justifyContent:'space-between', alignItems:'center', marginBottom:12}}>
        <h3 style={{margin:0}}>📧 Communications</h3>
        <button onClick={load} style={{background:'#21262d', color:'#e6edf3', border:'1px solid #30363d', padding:'6px 10px', borderRadius:8, cursor:'pointer'}}>Refresh</button>
      </div>

      {loading ? (
        <div style={{opacity:.6}}>Loading communications...</div>
      ) : (
        <div style={{display:'grid', gap:16}}>
          {/* Notification Digest */}
          <div style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:14}}>
            <h4 style={{margin:'0 0 8px', fontSize:14}}>📋 Notification Digest</h4>
            <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, margin:0, opacity:.9}}>
              {digest || 'No notifications'}
            </pre>
          </div>

          {/* Email Summary */}
          <div style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:14}}>
            <h4 style={{margin:'0 0 8px', fontSize:14}}>📧 Email Summary</h4>
            <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, margin:0, opacity:.9}}>
              {emailSummary || 'Email not configured. Set IMAP_HOST, IMAP_USER, IMAP_PASS environment variables for the comms-agent.'}
            </pre>
          </div>

          {/* Calendar */}
          <div style={{background:'#0d1117', border:'1px solid #30363d', borderRadius:8, padding:14}}>
            <h4 style={{margin:'0 0 8px', fontSize:14}}>📅 Calendar</h4>
            <pre style={{whiteSpace:'pre-wrap', fontFamily:'ui-monospace,monospace', fontSize:12, margin:0, opacity:.9}}>
              {calendarText || 'Calendar not configured. Set CALDAV_URL, CALDAV_USER, CALDAV_PASS environment variables for the comms-agent.'}
            </pre>
          </div>

          <div style={{fontSize:12, opacity:.6}}>
            Comms Agent: IMAP email · SMTP send · CalDAV calendar · LLM summarization · Auto-refresh: 60s
          </div>
        </div>
      )}
    </div>
  );
}
