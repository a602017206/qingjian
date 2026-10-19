import test from 'node:test';import {execFileSync} from 'node:child_process';
test('安卓原生库必须满足十六千字节段对齐并拒绝截断及错位',()=>{execFileSync('python3',['tests/android_package.py'],{stdio:'pipe'});});

test('安卓认证云端检查必须匹配进程阶段并拒绝失败标记',()=>{execFileSync('python3',['tests/android_credential_smoke.py'],{stdio:'pipe'});});
