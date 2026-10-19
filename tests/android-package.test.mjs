import test from 'node:test';import {execFileSync} from 'node:child_process';
test('安卓原生库必须满足十六千字节段对齐并拒绝截断及错位',()=>{execFileSync('python3',['tests/android_package.py'],{stdio:'pipe'});});
