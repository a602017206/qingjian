import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
test('前端安装清单与锁文件一致且移动流水线入口存在',async()=>{
 const p=JSON.parse(await readFile('package.json','utf8')),lock=JSON.parse(await readFile('package-lock.json','utf8'));
 assert.deepEqual(p.dependencies,lock.packages[''].dependencies);assert.equal(p.scripts['mobile:android'],'node scripts/mobile-ci.mjs android');assert.equal(p.scripts['mobile:ios'],'node scripts/mobile-ci.mjs ios');
});
