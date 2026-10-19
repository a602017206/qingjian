import test from 'node:test';import assert from 'node:assert/strict';
import { mobileBuildArgs } from '../scripts/mobile-ci.mjs';
test('移动构建只使用云端命令，安卓为发布包，苹果设备和模拟器明确区分',()=>{
 assert.deepEqual(mobileBuildArgs('android'),['android','build','--target','aarch64','--apk','--ci']);
 assert.ok(mobileBuildArgs('ios').includes('aarch64-sim'));assert.ok(mobileBuildArgs('ios-device').includes('aarch64'));assert.ok(mobileBuildArgs('ios-device').includes('--no-sign'));assert.ok(mobileBuildArgs('ios-device').includes('--archive-only'));assert.throws(()=>mobileBuildArgs('bad'));
});
