'use client';
import { useState, useEffect } from 'react';
import { getConfig, updateServiceConfig, updateSecret, testService, updatePreferences } from '../../lib/api';

const inputStyle: React.CSSProperties = {
  width: '100%', padding: '10px 14px', fontSize: 14,
  background: 'var(--bg-primary)', border: '1px solid var(--border)',
  borderRadius: 'var(--radius-sm)', color: 'var(--text-primary)',
  outline: 'none',
};

const btnStyle: React.CSSProperties = {
  padding: '8px 18px', fontSize: 13, fontWeight: 600,
  border: 'none', borderRadius: 'var(--radius-sm)', cursor: 'pointer',
};

const cardStyle: React.CSSProperties = {
  background: 'var(--bg-card)', border: '1px solid var(--border)',
  borderRadius: 'var(--radius-md)', padding: 20, marginBottom: 16,
};

const SERVICE_ICONS: Record<string, string> = {
  pihole: '🏗️', truenas: '💾', frigate: '📹', homeassistant: '🏠',
  ollama: '🧠', jellyfin: '🎵', searxng: '🔍', pfsense: '🔥',
  signal: '📱', mqtt: '📡', email: '✉️', calendar: '📅',
  social_twitter: '🐦', social_mastodon: '🐘', social_bluesky: '🦋',
};

export default function SettingsPage() {
  const [config, setConfig] = useState<any>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [tests, setTests] = useState<Record<string, any>>({});
  const [editValues, setEditValues] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');
  const [tab, setTab] = useState<'services' | 'preferences'>('services');

  useEffect(() => {
    getConfig().then(setConfig).catch(console.error);
  }, []);

  const doTest = async (service: string) => {
    setTests(prev => ({ ...prev, [service]: { testing: true } }));
    const result = await testService(service);
    setTests(prev => ({ ...prev, [service]: result }));
  };

  const saveService = async (service: string) => {
    setSaving(true);
    setMessage('');
    try {
      const svc = config.services[service];
      if (editValues[`${service}.url`] !== undefined) {
        await updateServiceConfig(service, { url: editValues[`${service}.url`] });
      }
      for (const [key, secretInfo] of Object.entries(svc.secrets || {} as Record<string, any>)) {
        const editKey = `${service}.secret.${key}`;
        if (editValues[editKey] && editValues[editKey] !== '••••••••') {
          await updateSecret(service, key, editValues[editKey]);
        }
      }
      setMessage(`✅ ${svc.name} saved`);
      const refreshed = await getConfig();
      setConfig(refreshed);
    } catch (e: any) {
      setMessage(`❌ Save failed: ${e.message}`);
    }
    setSaving(false);
  };

  const savePreferences = async () => {
    setSaving(true);
    try {
      const prefs: Record<string, any> = {};
      for (const [key, val] of Object.entries(editValues)) {
        if (key.startsWith('pref.')) {
          const prefKey = key.replace('pref.', '');
          prefs[prefKey] = val === 'true' ? true : val === 'false' ? false : isNaN(Number(val)) ? val : Number(val);
        }
      }
      await updatePreferences(prefs);
      setMessage('✅ Preferences saved');
      const refreshed = await getConfig();
      setConfig(refreshed);
    } catch (e: any) {
      setMessage(`❌ Save failed: ${e.message}`);
    }
    setSaving(false);
  };

  const getEditValue = (key: string, fallback: string) => editValues[key] ?? fallback;
  const setEdit = (key: string, value: string) => setEditValues(prev => ({ ...prev, [key]: value }));

  if (!config) return <div style={{ padding: 40, color: 'var(--text-secondary)' }}>Loading configuration...</div>;

  const services = config.services || {};
  const selectedSvc = selected ? services[selected] : null;

  return (
    <div style={{ display: 'flex', height: '100vh', background: 'var(--bg-primary)' }}>
      {/* Left sidebar */}
      <div style={{ width: 260, background: 'var(--bg-secondary)', borderRight: '1px solid var(--border)', overflowY: 'auto', padding: '16px 0' }}>
        <div style={{ padding: '8px 16px', marginBottom: 12 }}>
          <a href="/" style={{ color: 'var(--text-muted)', fontSize: 13, textDecoration: 'none' }}>← Dashboard</a>
          <h2 style={{ fontSize: 18, fontWeight: 700, color: 'var(--text-primary)', marginTop: 8 }}>⚙️ Settings</h2>
        </div>

        {/* Tab toggle */}
        <div style={{ display: 'flex', margin: '0 12px 12px', gap: 4 }}>
          <button onClick={() => setTab('services')} style={{ ...btnStyle, flex: 1, background: tab === 'services' ? 'var(--accent)' : 'var(--bg-elevated)', color: tab === 'services' ? '#fff' : 'var(--text-secondary)' }}>Services</button>
          <button onClick={() => setTab('preferences')} style={{ ...btnStyle, flex: 1, background: tab === 'preferences' ? 'var(--accent)' : 'var(--bg-elevated)', color: tab === 'preferences' ? '#fff' : 'var(--text-secondary)' }}>Prefs</button>
        </div>

        {tab === 'services' && Object.entries(services).map(([key, svc]: [string, any]) => {
          const test = tests[key];
          const statusDot = test?.ok ? 'var(--green)' : test?.ok === false ? 'var(--red)' : 'var(--text-muted)';
          return (
            <div
              key={key}
              onClick={() => { setSelected(key); setTab('services'); }}
              style={{
                padding: '10px 16px', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 10,
                background: selected === key ? 'var(--bg-elevated)' : 'transparent',
                borderLeft: selected === key ? '3px solid var(--accent)' : '3px solid transparent',
              }}
            >
              <span style={{ fontSize: 18 }}>{SERVICE_ICONS[key] || '🔧'}</span>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: 14, color: 'var(--text-primary)' }}>{svc.name}</div>
                <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>{svc.url || 'Not configured'}</div>
              </div>
              <div style={{ width: 8, height: 8, borderRadius: '50%', background: statusDot }} />
            </div>
          );
        })}
      </div>

      {/* Right panel */}
      <div style={{ flex: 1, overflowY: 'auto', padding: 32 }}>
        {message && (
          <div style={{ padding: '10px 16px', marginBottom: 16, borderRadius: 'var(--radius-sm)', background: message.startsWith('✅') ? 'var(--green-dim)' : 'var(--red-dim)', color: message.startsWith('✅') ? 'var(--green)' : 'var(--red)', fontSize: 13 }}>
            {message}
          </div>
        )}

        {tab === 'services' && selectedSvc && (
          <div>
            <h2 style={{ fontSize: 20, fontWeight: 600, marginBottom: 4 }}>
              {SERVICE_ICONS[selected!] || '🔧'} {selectedSvc.name}
            </h2>
            <p style={{ color: 'var(--text-muted)', fontSize: 13, marginBottom: 24 }}>
              Agent: {selectedSvc.agent || 'system'}
            </p>

            <div style={cardStyle}>
              <h3 style={{ fontSize: 15, marginBottom: 16 }}>Connection</h3>
              <div style={{ marginBottom: 16 }}>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>URL</label>
                <div style={{ display: 'flex', gap: 8 }}>
                  <input
                    value={getEditValue(`${selected}.url`, selectedSvc.url)}
                    onChange={e => setEdit(`${selected}.url`, e.target.value)}
                    style={{ ...inputStyle, flex: 1 }}
                  />
                  <button onClick={() => doTest(selected!)} style={{ ...btnStyle, background: 'var(--bg-elevated)', color: 'var(--text-primary)' }}>
                    {tests[selected!]?.testing ? '...' : 'Test'}
                  </button>
                </div>
                {tests[selected!] && !tests[selected!].testing && (
                  <div style={{ marginTop: 6, fontSize: 12, color: tests[selected!].ok ? 'var(--green)' : 'var(--red)' }}>
                    {tests[selected!].ok ? `✅ Connected (${tests[selected!].latency_ms}ms)` : `❌ ${tests[selected!].error || 'Not reachable'}`}
                  </div>
                )}
              </div>
            </div>

            {selectedSvc.secrets && Object.keys(selectedSvc.secrets).length > 0 && (
              <div style={cardStyle}>
                <h3 style={{ fontSize: 15, marginBottom: 16 }}>Secrets</h3>
                {Object.entries(selectedSvc.secrets).map(([key, info]: [string, any]) => (
                  <div key={key} style={{ marginBottom: 16 }}>
                    <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>
                      {key} {info.configured && <span style={{ color: 'var(--green)', fontSize: 11 }}>✓ configured</span>}
                    </label>
                    <input
                      type="password"
                      value={getEditValue(`${selected}.secret.${key}`, info.configured ? '••••••••' : '')}
                      onChange={e => setEdit(`${selected}.secret.${key}`, e.target.value)}
                      placeholder={`Enter ${key}`}
                      style={inputStyle}
                    />
                  </div>
                ))}
              </div>
            )}

            <button onClick={() => saveService(selected!)} disabled={saving} style={{ ...btnStyle, background: 'var(--accent)', color: '#fff' }}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        )}

        {tab === 'services' && !selectedSvc && (
          <div style={{ color: 'var(--text-muted)', textAlign: 'center', marginTop: 100, fontSize: 15 }}>
            ← Select a service to configure
          </div>
        )}

        {tab === 'preferences' && (
          <div>
            <h2 style={{ fontSize: 20, fontWeight: 600, marginBottom: 24 }}>🎛️ Preferences</h2>

            <div style={cardStyle}>
              <h3 style={{ fontSize: 15, marginBottom: 16 }}>🔔 Notifications</h3>
              <div style={{ display: 'flex', gap: 16, marginBottom: 12 }}>
                <div style={{ flex: 1 }}>
                  <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Quiet Start (hour)</label>
                  <input type="number" min={0} max={23} value={getEditValue('pref.notify_quiet_start', String(config.preferences?.notify_quiet_start ?? 22))} onChange={e => setEdit('pref.notify_quiet_start', e.target.value)} style={inputStyle} />
                </div>
                <div style={{ flex: 1 }}>
                  <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Quiet End (hour)</label>
                  <input type="number" min={0} max={23} value={getEditValue('pref.notify_quiet_end', String(config.preferences?.notify_quiet_end ?? 7))} onChange={e => setEdit('pref.notify_quiet_end', e.target.value)} style={inputStyle} />
                </div>
              </div>
              <div style={{ marginBottom: 12 }}>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Channel</label>
                <input value={getEditValue('pref.notify_channel', config.preferences?.notify_channel || 'signal')} onChange={e => setEdit('pref.notify_channel', e.target.value)} style={inputStyle} />
              </div>
            </div>

            <div style={cardStyle}>
              <h3 style={{ fontSize: 15, marginBottom: 16 }}>🎤 Voice</h3>
              <label style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12, cursor: 'pointer' }}>
                <input type="checkbox" checked={getEditValue('pref.voice_enabled', String(config.preferences?.voice_enabled ?? false)) === 'true'} onChange={e => setEdit('pref.voice_enabled', String(e.target.checked))} />
                <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>Enable voice interface</span>
              </label>
              <div>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Wake Word</label>
                <input value={getEditValue('pref.voice_wake_word', config.preferences?.voice_wake_word || 'hey_jarvis')} onChange={e => setEdit('pref.voice_wake_word', e.target.value)} style={inputStyle} />
              </div>
            </div>

            <div style={cardStyle}>
              <h3 style={{ fontSize: 15, marginBottom: 16 }}>🖥️ Display</h3>
              <label style={{ display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer' }}>
                <input type="checkbox" checked={getEditValue('pref.gui_enabled', String(config.preferences?.gui_enabled ?? false)) === 'true'} onChange={e => setEdit('pref.gui_enabled', String(e.target.checked))} />
                <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>Enable kiosk display (fullscreen dashboard on monitor)</span>
              </label>
            </div>

            <div style={cardStyle}>
              <h3 style={{ fontSize: 15, marginBottom: 16 }}>🌍 Location</h3>
              <div>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Weather Location</label>
                <input value={getEditValue('pref.weather_location', config.preferences?.weather_location || '')} onChange={e => setEdit('pref.weather_location', e.target.value)} placeholder="City, Country" style={inputStyle} />
              </div>
            </div>

            <button onClick={savePreferences} disabled={saving} style={{ ...btnStyle, background: 'var(--accent)', color: '#fff' }}>
              {saving ? 'Saving...' : 'Save Preferences'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
