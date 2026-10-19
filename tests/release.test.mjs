import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,writeFile,mkdir,copyFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkVersion,releaseAssets,verifyAssets } from '../scripts/release.mjs';
test('首版标签与前端原生配置锁文件版本一致，不接受错配标签',async()=>{
 const version=JSON.parse(await readFile('package.json','utf8')).version;assert.equal(await checkVersion(process.cwd(),'v'+version),version);
 await assert.rejects(checkVersion(process.cwd(),'v9.9.9'),/版本/);
});
test('发布必须包含所有平台真实产物，摘要可校验且篡改后拒绝',async()=>{
 const root=await mkdtemp(join(tmpdir(),'qingjian-release-'));
 for(const name of releaseAssets)await writeFile(join(root,name),'真实测试产物 '+name);
 const entries=await verifyAssets(root);assert.equal(entries.length,releaseAssets.length);
 const digest=await readFile(join(root,'SHA256SUMS.txt'),'utf8');assert.ok(digest.includes('qingjian-android-arm64.apk'));
 await writeFile(join(root,releaseAssets[0]),'');await assert.rejects(verifyAssets(root),/空|缺少/);await writeFile(join(root,releaseAssets[0]),'恢复');await writeFile(join(root,'private.env'),'保密');await assert.rejects(verifyAssets(root),/非发布/);
});
test('微软桌面检出为回车换行后仍能正确核对锁文件版本',async()=>{
 const root=await mkdtemp(join(tmpdir(),'qingjian-crlf-'));
 for(const path of ['package.json','package-lock.json','src-tauri/tauri.conf.json','src-tauri/Cargo.toml','crates/notebook/Cargo.toml','Cargo.lock']){const target=join(root,path);await mkdir(join(target,'..'),{recursive:true});await writeFile(target,(await readFile(path,'utf8')).replace(/\r?\n/g,'\r\n'));}
 const version=JSON.parse(await readFile('package.json','utf8')).version;assert.equal(await checkVersion(root,'v'+version),version);
});
