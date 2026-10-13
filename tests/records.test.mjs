import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp,readFile,readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { appendRecord } from '../scripts/record-change.mjs';

test('当天记录追加到同一文件并以北京时间命名',async()=>{
 const root=await mkdtemp(join(tmpdir(),'qingjian-record-'));
 const first=await appendRecord(root,'存储设计','增加本地存储',new Date('2026-10-13T01:00:00Z'));
 const second=await appendRecord(root,'问题修复','修复版本冲突',new Date('2026-10-13T02:00:00Z'));
 assert.equal(first,second);assert.match(first,/2026-10-13_0900_存储设计\.md$/);
 const files=await readdir(root);assert.equal(files.length,1);const text=await readFile(first,'utf8');assert.match(text,/增加本地存储/);assert.match(text,/修复版本冲突/);
 const next=await appendRecord(root,'联调验证','记录测试结果',new Date('2026-10-14T01:00:00Z'));assert.notEqual(next,first);
});
