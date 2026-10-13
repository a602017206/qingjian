import { spawnSync } from 'node:child_process';
if(process.env.GITHUB_ACTIONS!=='true'){
 console.error('按项目约定，移动端原生编译和构建只能在 GitHub 流水线执行。');
 process.exit(1);
}
const platform=process.argv[2];
if(!['android','ios'].includes(platform))throw new Error('移动端平台无效');
const args=platform==='android'?['android','build','--debug','--target','aarch64','--apk','--ci']:['ios','build','--debug','--target','aarch64-sim','--no-sign','--ci'];
const result=spawnSync(process.platform==='win32'?'npm.cmd':'npm',['run','tauri','--',...args],{stdio:'inherit',env:process.env});
if(result.error)throw result.error;
process.exit(result.status??1);
