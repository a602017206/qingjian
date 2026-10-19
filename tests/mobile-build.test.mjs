import test from 'node:test';import assert from 'node:assert/strict';
import { mobileBuildArgs } from '../scripts/mobile-ci.mjs';
test('移动构建只使用云端命令，安卓为发布包，苹果设备和模拟器明确区分',()=>{
 assert.deepEqual(mobileBuildArgs('android'),['android','build','--target','aarch64','--apk','--ci']);
 assert.ok(mobileBuildArgs('ios').includes('aarch64-sim'));assert.ok(mobileBuildArgs('ios-device').includes('aarch64'));assert.ok(mobileBuildArgs('ios-device').includes('--no-sign'));assert.ok(mobileBuildArgs('ios-device').includes('--archive-only'));assert.throws(()=>mobileBuildArgs('bad'));
});
test('苹果移动构建明确继承系统字符转换链接配置，安卓不注入苹果参数',async()=>{
 const { mobileBuildEnvironment }=await import('../scripts/mobile-ci.mjs');const inherited={GITHUB_ACTIONS:'true',OTHER_SETTING:'kept'};
 assert.deepEqual(mobileBuildEnvironment('android',inherited),inherited);for(const platform of ['ios','ios-device']){const env=mobileBuildEnvironment(platform,inherited);assert.equal(env.OTHER_SETTING,'kept');assert.ok(env.XCODE_XCCONFIG_FILE.endsWith('/src-tauri/ios-link.xcconfig'));const {readFile}=await import('node:fs/promises');assert.match(await readFile(env.XCODE_XCCONFIG_FILE,'utf8'),/OTHER_LDFLAGS = \$\(inherited\) -liconv/);}
});
