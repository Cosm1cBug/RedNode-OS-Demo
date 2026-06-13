export const metadata = { title: 'RedNode-OS', description: 'Personal Autonomous Operating System' };
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (<html lang="en"><body style={{fontFamily:'ui-sans-serif,system-ui', background:'#0b0f14', color:'#e6edf3', margin:0}}>{children}</body></html>);
}
