export const metadata = { title: 'RedNode-OS', description: 'Personal Autonomous Operating System' };
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <style>{`
          * { box-sizing: border-box; margin: 0; padding: 0; }
          :root {
            --bg-primary: #0a0e14;
            --bg-secondary: #0f1419;
            --bg-card: #131920;
            --bg-elevated: #1a2028;
            --border: #1e2a36;
            --border-hover: #2d3e4f;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --text-muted: #6e7681;
            --accent: #1f6feb;
            --accent-hover: #388bfd;
            --green: #2ea043;
            --green-dim: rgba(46,160,67,0.15);
            --red: #f85149;
            --red-dim: rgba(248,81,73,0.15);
            --amber: #d29922;
            --amber-dim: rgba(210,153,34,0.15);
            --blue: #1f6feb;
            --blue-dim: rgba(31,111,235,0.15);
            --purple: #8957e5;
            --radius-sm: 6px;
            --radius-md: 10px;
            --radius-lg: 14px;
            --shadow: 0 2px 8px rgba(0,0,0,0.3);
            --font-sans: 'Inter', ui-sans-serif, system-ui, -apple-system, sans-serif;
            --font-mono: 'JetBrains Mono', 'Fira Code', ui-monospace, monospace;
            --transition: 0.15s ease;
          }
          body {
            font-family: var(--font-sans);
            background: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.5;
            -webkit-font-smoothing: antialiased;
          }
          button {
            font-family: var(--font-sans);
            cursor: pointer;
            transition: all var(--transition);
          }
          button:hover { filter: brightness(1.1); }
          pre, code { font-family: var(--font-mono); }
          ::-webkit-scrollbar { width: 6px; height: 6px; }
          ::-webkit-scrollbar-track { background: transparent; }
          ::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
          ::-webkit-scrollbar-thumb:hover { background: var(--text-muted); }

          /* Animations */
          @keyframes fadeIn { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
          @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
          .animate-in { animation: fadeIn 0.2s ease-out; }
          .pulse { animation: pulse 2s infinite; }
        `}</style>
      </head>
      <body>{children}</body>
    </html>
  );
}
