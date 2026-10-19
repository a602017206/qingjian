import test from 'node:test';
import assert from 'node:assert/strict';
import { runDesktopBuild } from '../scripts/desktop-build.mjs';
test('苹果桌面先构建再签名打包，其他桌面继续使用原生目标',()=>{
 const calls=[],run=(command,args)=>{calls.push({command,args});return {status:0};};
 assert.equal(runDesktopBuild({platform:'darwin',run}),0);
 assert.equal(calls.length,2);
 assert.ok(calls[0].args.includes('app'));
 assert.ok(calls[0].args.includes('--locked'));
 assert.equal(calls[1].command,'python3');
 assert.ok(calls[1].args[0].endsWith('package-macos.py'));
 calls.length=0;assert.equal(runDesktopBuild({platform:'win32',run}),0);assert.equal(calls.length,1);
});
test('构建或签名校验失败会返回失败状态，不继续后续阶段',()=>{
 let count=0;assert.equal(runDesktopBuild({platform:'darwin',run:()=>{count++;return {status:7};}}),7);assert.equal(count,1);
 count=0;assert.equal(runDesktopBuild({platform:'darwin',run:()=>({status:++count===1?0:9})}),9);assert.equal(count,2);
});
