import test from 'node:test';import assert from 'node:assert/strict';
import { mobileBuildArgs } from '../scripts/mobile-ci.mjs';
test('移动构建只使用云端命令，安卓为发布包，苹果设备和模拟器明确区分',()=>{
 assert.deepEqual(mobileBuildArgs('android'),['android','build','--target','aarch64','--apk','--ci']);
 assert.ok(mobileBuildArgs('ios').includes('aarch64-sim'));assert.ok(mobileBuildArgs('ios-device').includes('aarch64'));assert.ok(mobileBuildArgs('ios-device').includes('--no-sign'));assert.ok(mobileBuildArgs('ios-device').includes('--archive-only'));assert.throws(()=>mobileBuildArgs('bad'));
});
test('苹果系统库写入生成工程，不依赖会被框架过滤的环境变量',async()=>{
 const {addIosSystemLibrary}=await import('../scripts/configure-ios.mjs');const source='targets:\n  app_iOS:\n    dependencies:\n      - sdk: Security.framework\n      - sdk: WebKit.framework\n';const result=addIosSystemLibrary(source);assert.match(result,/      - sdk: libiconv.tbd\n      - sdk: Security.framework/);assert.equal(addIosSystemLibrary(result),result);assert.throws(()=>addIosSystemLibrary('unexpected: template'),/系统库/);assert.equal(addIosSystemLibrary(source.replaceAll('\n','\r\n')).replaceAll('\r\n','\n'),result);
});
