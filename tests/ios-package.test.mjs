import test from 'node:test';import { execFileSync } from 'node:child_process';
test('苹果模拟器与未签名真机包不会混用，压缩保留执行权限',()=>execFileSync('python3',['tests/package_ios.py'],{stdio:'pipe'}));
