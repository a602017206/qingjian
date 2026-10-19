import { spawnSync } from 'node:child_process';
import { resolve,dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
export function runDesktopBuild({platform=process.platform,run=spawnSync}={}){
 const build=[resolve(root,'node_modules/@tauri-apps/cli/tauri.js'),'build',...(platform==='darwin'?['--bundles','app']:[]),'--','--locked'];
 const steps=[[process.execPath,build],...(platform==='darwin'?[['python3',[resolve(root,'scripts/package-macos.py')]]]:[])];
 for(const [command,args] of steps){const result=run(command,args,{cwd:root,stdio:'inherit',env:process.env});if(result.error)throw result.error;if(result.status!==0)return result.status??1;}
 return 0;
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{if(process.argv.length>2)throw new Error('桌面打包命令不接受额外参数，请在项目构建配置中设置目标。');process.exitCode=runDesktopBuild();}
 catch(error){console.error('桌面打包未完成：'+error.message);process.exitCode=1;}
}
