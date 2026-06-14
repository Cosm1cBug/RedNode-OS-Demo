// RedNode-OS Desktop — Tauri
// Loads the Next.js web dashboard in a native window.
// In dev: connects to http://localhost:3000
// In production: loads bundled frontend from dist/

const { invoke } = (window as any).__TAURI__.core;

async function init() {
  try {
    const health = await invoke('cns_health');
    console.log('RedNode CNS:', health);
  } catch (e) {
    console.warn('RedNode CNS not reachable:', e);
  }
}

init();
