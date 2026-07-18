'use client';
import { useState } from 'react';
import { updateServiceConfig, updateSecret, testService, updatePreferences } from '../../lib/api';

const STEPS = [
  { id: 'welcome', label: 'Welcome' },
  { id: 'network', label: 'Network Services' },
  { id: 'home', label: 'Home Automation' },
  { id: 'media', label: 'Media & Comms' },
  { id: 'preferences', label: 'Preferences' },
  { id: 'done', label: 'Complete' },
];

const inputStyle: React.CSSProperties = {
  width: '100%', padding: '10px 14px', fontSize: 14,
  background: 'var(--bg-primary)', border: '1px solid var(--border)',
  borderRadius: 'var(--radius-sm)', color: 'var(--text-primary)',
  outline: 'none',
};

const btnStyle: React.CSSProperties = {
  padding: '10px 24px', fontSize: 14, fontWeight: 600,
  background: 'var(--accent)', color: '#fff', border: 'none',
  borderRadius: 'var(--radius-sm)', cursor: 'pointer',
};

const cardStyle: React.CSSProperties = {
  background: 'var(--bg-card)', border: '1px solid var(--border)',
  borderRadius: 'var(--radius-md)', padding: 20, marginBottom: 16,
};

function ServiceField({ label, placeholder, value, onChange, type = 'text', onTest, testResult }: {
  label: string; placeholder: string; value: string;
  onChange: (v: string) => void; type?: string;
  onTest?: () => void; testResult?: any;
}) {
  return (
    <div style={{ marginBottom: 16 }}>
      <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>{label}</label>
      <div style={{ display: 'flex', gap: 8 }}>
        <input
          type={type} value={value} onChange={e => onChange(e.target.value)}
          placeholder={placeholder} style={{ ...inputStyle, flex: 1 }}
        />
        {onTest && (
          <button onClick={onTest} style={{ ...btnStyle, background: 'var(--bg-elevated)', color: 'var(--text-primary)', fontSize: 13, padding: '8px 16px' }}>
            Test
          </button>
        )}
      </div>
      {testResult && (
        <div style={{ marginTop: 6, fontSize: 12, color: testResult.ok ? 'var(--green)' : 'var(--red)' }}>
          {testResult.ok ? `✅ Connected (${testResult.latency_ms}ms)` : `❌ ${testResult.error || 'Not reachable'}`}
        </div>
      )}
    </div>
  );
}

export default function SetupWizard() {
  const [step, setStep] = useState(0);
  const [config, setConfig] = useState<Record<string, Record<string, string>>>({
    pihole: { url: 'http://10.0.50.2', password: '' },
    truenas: { url: 'https://10.0.50.3', api_key: '' },
    frigate: { url: 'http://localhost:5000' },
    homeassistant: { url: 'http://localhost:8123', token: '' },
    ollama: { url: 'http://127.0.0.1:11434' },
    jellyfin: { url: 'http://localhost:8096', api_key: '' },
    pfsense: { url: 'https://10.0.50.1', api_key: '', api_secret: '' },
    signal: { bot_number: '', owner_number: '' },
  });
  const [prefs, setPrefs] = useState({
    notify_quiet_start: 22, notify_quiet_end: 7, notify_channel: 'signal',
    voice_enabled: false, voice_wake_word: 'hey_jarvis',
    weather_location: '', gui_enabled: false,
  });
  const [tests, setTests] = useState<Record<string, any>>({});
  const [saving, setSaving] = useState(false);

  const updateField = (service: string, field: string, value: string) => {
    setConfig(prev => ({ ...prev, [service]: { ...prev[service], [field]: value } }));
  };

  const doTest = async (service: string) => {
    const result = await testService(service);
    setTests(prev => ({ ...prev, [service]: result }));
  };

  const saveAll = async () => {
    setSaving(true);
    try {
      for (const [svc, fields] of Object.entries(config)) {
        if (fields.url) await updateServiceConfig(svc, { url: fields.url });
        for (const [key, val] of Object.entries(fields)) {
          if (key !== 'url' && val) {
            const envKey = key === 'password' ? `${svc.toUpperCase()}_PASSWORD` :
                          key === 'api_key' ? `${svc.toUpperCase()}_API_KEY` :
                          key === 'token' ? `${svc.toUpperCase()}_TOKEN` :
                          key === 'api_secret' ? `${svc.toUpperCase()}_API_SECRET` :
                          key === 'bot_number' ? 'SIGNAL_BOT_NUMBER' :
                          key === 'owner_number' ? 'SIGNAL_OWNER_NUMBER' :
                          `${svc.toUpperCase()}_${key.toUpperCase()}`;
            await updateSecret(svc, envKey, val);
          }
        }
      }
      await updatePreferences(prefs);
    } catch (e) {
      console.error('Save failed:', e);
    }
    setSaving(false);
  };

  const current = STEPS[step];

  return (
    <div style={{ maxWidth: 700, margin: '0 auto', padding: '40px 20px' }}>
      <div style={{ textAlign: 'center', marginBottom: 40 }}>
        <h1 style={{ fontSize: 28, fontWeight: 700, color: 'var(--text-primary)' }}>🧠 RedNode-OS Setup</h1>
        <p style={{ color: 'var(--text-secondary)', marginTop: 8 }}>Configure your services. You can change these later in Settings.</p>
      </div>

      {/* Progress bar */}
      <div style={{ display: 'flex', gap: 4, marginBottom: 32 }}>
        {STEPS.map((s, i) => (
          <div key={s.id} style={{
            flex: 1, height: 4, borderRadius: 2,
            background: i <= step ? 'var(--accent)' : 'var(--border)',
          }} />
        ))}
      </div>

      {/* Step content */}
      {current.id === 'welcome' && (
        <div style={cardStyle}>
          <h2 style={{ fontSize: 20, marginBottom: 12 }}>Welcome to RedNode-OS</h2>
          <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: 16 }}>
            This wizard will help you connect your home infrastructure to RedNode.
            Enter your service URLs and API keys — RedNode will test each connection
            to make sure everything works.
          </p>
          <p style={{ color: 'var(--text-muted)', fontSize: 13 }}>
            Skip any service you don't have — you can add it later in Settings.
          </p>
        </div>
      )}

      {current.id === 'network' && (
        <div>
          <h2 style={{ fontSize: 18, marginBottom: 16 }}>Network Services</h2>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🏗️ Pi-hole DNS</h3>
            <ServiceField label="Pi-hole URL" placeholder="http://10.0.50.2" value={config.pihole?.url || ''} onChange={v => updateField('pihole', 'url', v)} onTest={() => doTest('pihole')} testResult={tests.pihole} />
            <ServiceField label="Admin Password" placeholder="Pi-hole admin password" value={config.pihole?.password || ''} onChange={v => updateField('pihole', 'password', v)} type="password" />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🔥 pfSense Firewall</h3>
            <ServiceField label="pfSense URL" placeholder="https://10.0.50.1" value={config.pfsense?.url || ''} onChange={v => updateField('pfsense', 'url', v)} onTest={() => doTest('pfsense')} testResult={tests.pfsense} />
            <ServiceField label="API Key" placeholder="pfSense API key" value={config.pfsense?.api_key || ''} onChange={v => updateField('pfsense', 'api_key', v)} type="password" />
            <ServiceField label="API Secret" placeholder="pfSense API secret" value={config.pfsense?.api_secret || ''} onChange={v => updateField('pfsense', 'api_secret', v)} type="password" />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>💾 TrueNAS Storage</h3>
            <ServiceField label="TrueNAS URL" placeholder="https://10.0.50.3" value={config.truenas?.url || ''} onChange={v => updateField('truenas', 'url', v)} onTest={() => doTest('truenas')} testResult={tests.truenas} />
            <ServiceField label="API Key" placeholder="TrueNAS API key" value={config.truenas?.api_key || ''} onChange={v => updateField('truenas', 'api_key', v)} type="password" />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🧠 Ollama LLM</h3>
            <ServiceField label="Ollama URL" placeholder="http://127.0.0.1:11434" value={config.ollama?.url || ''} onChange={v => updateField('ollama', 'url', v)} onTest={() => doTest('ollama')} testResult={tests.ollama} />
          </div>
        </div>
      )}

      {current.id === 'home' && (
        <div>
          <h2 style={{ fontSize: 18, marginBottom: 16 }}>Home Automation</h2>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🏠 Home Assistant</h3>
            <ServiceField label="Home Assistant URL" placeholder="http://localhost:8123" value={config.homeassistant?.url || ''} onChange={v => updateField('homeassistant', 'url', v)} onTest={() => doTest('homeassistant')} testResult={tests.homeassistant} />
            <ServiceField label="Long-Lived Access Token" placeholder="HA token" value={config.homeassistant?.token || ''} onChange={v => updateField('homeassistant', 'token', v)} type="password" />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>📹 Frigate NVR</h3>
            <ServiceField label="Frigate URL" placeholder="http://localhost:5000" value={config.frigate?.url || ''} onChange={v => updateField('frigate', 'url', v)} onTest={() => doTest('frigate')} testResult={tests.frigate} />
          </div>
        </div>
      )}

      {current.id === 'media' && (
        <div>
          <h2 style={{ fontSize: 18, marginBottom: 16 }}>Media & Communications</h2>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🎵 Jellyfin Media Server</h3>
            <ServiceField label="Jellyfin URL" placeholder="http://localhost:8096" value={config.jellyfin?.url || ''} onChange={v => updateField('jellyfin', 'url', v)} onTest={() => doTest('jellyfin')} testResult={tests.jellyfin} />
            <ServiceField label="API Key" placeholder="Jellyfin API key" value={config.jellyfin?.api_key || ''} onChange={v => updateField('jellyfin', 'api_key', v)} type="password" />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>📱 Signal Bot</h3>
            <ServiceField label="Bot Phone Number" placeholder="+1234567890" value={config.signal?.bot_number || ''} onChange={v => updateField('signal', 'bot_number', v)} />
            <ServiceField label="Your Phone Number (owner)" placeholder="+1234567890" value={config.signal?.owner_number || ''} onChange={v => updateField('signal', 'owner_number', v)} />
          </div>
        </div>
      )}

      {current.id === 'preferences' && (
        <div>
          <h2 style={{ fontSize: 18, marginBottom: 16 }}>Preferences</h2>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🔔 Notifications</h3>
            <div style={{ display: 'flex', gap: 16, marginBottom: 12 }}>
              <div style={{ flex: 1 }}>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Quiet Start (hour)</label>
                <input type="number" min={0} max={23} value={prefs.notify_quiet_start} onChange={e => setPrefs(p => ({ ...p, notify_quiet_start: +e.target.value }))} style={inputStyle} />
              </div>
              <div style={{ flex: 1 }}>
                <label style={{ display: 'block', fontSize: 13, color: 'var(--text-secondary)', marginBottom: 6 }}>Quiet End (hour)</label>
                <input type="number" min={0} max={23} value={prefs.notify_quiet_end} onChange={e => setPrefs(p => ({ ...p, notify_quiet_end: +e.target.value }))} style={inputStyle} />
              </div>
            </div>
            <ServiceField label="Notification Channel" placeholder="signal" value={prefs.notify_channel} onChange={v => setPrefs(p => ({ ...p, notify_channel: v }))} />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🎤 Voice</h3>
            <label style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12, cursor: 'pointer' }}>
              <input type="checkbox" checked={prefs.voice_enabled} onChange={e => setPrefs(p => ({ ...p, voice_enabled: e.target.checked }))} />
              <span style={{ fontSize: 14, color: 'var(--text-primary)' }}>Enable voice interface (mic + wake word + speakers)</span>
            </label>
            <ServiceField label="Wake Word" placeholder="hey_jarvis" value={prefs.voice_wake_word} onChange={v => setPrefs(p => ({ ...p, voice_wake_word: v }))} />
          </div>
          <div style={cardStyle}>
            <h3 style={{ fontSize: 15, marginBottom: 12 }}>🌍 Location</h3>
            <ServiceField label="Weather Location" placeholder="Kondotty, Kerala" value={prefs.weather_location} onChange={v => setPrefs(p => ({ ...p, weather_location: v }))} />
          </div>
        </div>
      )}

      {current.id === 'done' && (
        <div style={cardStyle}>
          <h2 style={{ fontSize: 20, marginBottom: 12 }}>✅ Setup Complete</h2>
          <p style={{ color: 'var(--text-secondary)', lineHeight: 1.6, marginBottom: 16 }}>
            RedNode-OS is configured. Your settings are saved and agents are being notified.
            You can change any setting later from the Settings page.
          </p>
          <p style={{ color: 'var(--text-muted)', fontSize: 13 }}>
            Dashboard: <a href="/" style={{ color: 'var(--accent)' }}>Open Dashboard →</a>
          </p>
        </div>
      )}

      {/* Navigation */}
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 24 }}>
        <button
          onClick={() => setStep(s => Math.max(0, s - 1))}
          disabled={step === 0}
          style={{ ...btnStyle, background: step === 0 ? 'var(--bg-elevated)' : 'var(--bg-card)', color: 'var(--text-secondary)', opacity: step === 0 ? 0.5 : 1 }}
        >
          ← Back
        </button>

        {step < STEPS.length - 1 ? (
          <button onClick={() => setStep(s => s + 1)} style={btnStyle}>
            {step === STEPS.length - 2 ? 'Finish' : 'Next →'}
          </button>
        ) : (
          <button onClick={() => { saveAll(); window.location.href = '/'; }} disabled={saving} style={btnStyle}>
            {saving ? 'Saving...' : 'Go to Dashboard →'}
          </button>
        )}
      </div>
    </div>
  );
}
