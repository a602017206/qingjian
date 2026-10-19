import test from 'node:test';
import { execFileSync } from 'node:child_process';
test('苹果分发包说明、签名校验及失败保留旧包',()=>{
 execFileSync('python3',['tests/package_macos.py'],{stdio:'pipe'});
});
