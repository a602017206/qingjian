import test from 'node:test';
import assert from 'node:assert/strict';
import { pickedFilename } from '../ui/web/core.mjs';

test('桌面文件名的百分号不触发解码错误',()=>{
 assert.equal(pickedFilename('/notes/100%完成.md'),'100%完成.md');
 assert.equal(pickedFilename('file:///notes/%E4%B8%AD%E6%96%87.md'),'中文.md');
 assert.equal(pickedFilename('content://downloads/document/123'),'123');
});
