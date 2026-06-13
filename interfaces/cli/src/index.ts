#!/usr/bin/env node
import { Command } from 'commander';
const program = new Command();
const CNS = process.env.REDNODE_CNS || 'http://localhost:8787';
program.name('rednode').description('RedNode-OS CLI').version('0.2.0');
program.command('intent <text...>').description('Submit intention')
  .action(async (text)=>{
    const intent = text.join(' ');
    const res = await fetch(`${CNS}/intent`, {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify({intent})});
    console.log(JSON.stringify(await res.json(), null, 2));
  });
program.command('health').action(async ()=>{ const r=await fetch(`${CNS}/health`); console.log(await r.json()) });
program.command('agents').action(()=> console.log(['system-agent','security-agent','coding-agent','research-agent','automation-agent','network-agent'].join('\n')));
program.parse();
