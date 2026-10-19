import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
test('前端安装清单与锁文件一致且移动流水线入口存在',async()=>{
 const p=JSON.parse(await readFile('package.json','utf8')),lock=JSON.parse(await readFile('package-lock.json','utf8'));
 assert.deepEqual(p.dependencies,lock.packages[''].dependencies);assert.equal(p.scripts['mobile:android'],'node scripts/mobile-ci.mjs android');assert.equal(p.scripts['mobile:ios'],'node scripts/mobile-ci.mjs ios');
});
test('安卓云端初始化明确选择平台工具，避免安装已移除的旧工具包',async()=>{
 const workflow=await readFile('.github/workflows/mobile.yml','utf8');assert.match(workflow,/uses: android-actions\/setup-android@v3\s+with:\s+packages: platform-tools/);
});
test('安卓原生依赖使用同一套原生归档与索引工具',async()=>{
 const workflow=await readFile('.github/workflows/mobile.yml','utf8');assert.match(workflow,/RANLIB_aarch64_linux_android=.*\/llvm-ranlib/);
});
test('苹果移动端静态嵌入压缩依赖，避免原生归档遗漏系统库链接',async()=>{
 const manifest=await readFile('crates/notebook/Cargo.toml','utf8');assert.match(manifest,/\[target\.'cfg\(target_os = "ios"\)'\.dependencies\]\s+libz-sys = \{ version = "1", features = \["static"\] \}/);
});
test('安卓实际链接设置同时约束最大页与通用页，防止只有压缩包对齐',async()=>{
 const build=await readFile('src-tauri/build.rs','utf8');assert.match(build,/max-page-size=16384/);assert.match(build,/common-page-size=16384/);
});
