import { build } from 'esbuild';
import { mkdir,copyFile } from 'node:fs/promises';
await mkdir('dist',{recursive:true});
await build({entryPoints:['ui/web/app.mjs'],outfile:'dist/app.mjs',bundle:true,format:'esm',target:['es2022'],minify:true,legalComments:'none'});
for(const file of ['index.html','style.css'])await copyFile(`ui/web/${file}`,`dist/${file}`);
console.log('共享界面构建完成');
