import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
export function mobileBuildArgs(platform){
 if(platform==='android')return ['android','build','--target','aarch64','--apk','--ci'];
 if(platform==='ios'||platform==='ios-device')return ['ios','build','--target',platform==='ios'?'aarch64-sim':'aarch64','--no-sign','--archive-only','--ci'];
 throw new Error('移动端平台无效');
}
export function mobileBuildEnvironment(platform,env=process.env){
 mobileBuildArgs(platform);
 return platform==='android'?{...env}:{...env,XCODE_XCCONFIG_FILE:fileURLToPath(new URL('../src-tauri/ios-link.xcconfig',import.meta.url))};
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 if(process.env.GITHUB_ACTIONS!=='true'){console.error('按项目约定，移动端原生编译和构建只能在 GitHub 流水线执行。');process.exitCode=1;}
 else{const result=spawnSync(process.execPath,[resolve('node_modules/@tauri-apps/cli/tauri.js'),...mobileBuildArgs(process.argv[2])],{stdio:'inherit',env:mobileBuildEnvironment(process.argv[2])});if(result.error)throw result.error;process.exitCode=result.status??1;}
}
