import { execFileSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
for(const file of readdirSync('ui/web').filter(f=>f.endsWith('.mjs')))execFileSync(process.execPath,['--check',`ui/web/${file}`]);
console.log('前端语法检查通过');
