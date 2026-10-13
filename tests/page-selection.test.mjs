import test from 'node:test';import assert from 'node:assert/strict';
const m=await import('../ui/web/page-selection.mjs').catch(()=>({}));
test('批量选择去重并包含子页面，忽略另一状态的页面',()=>{assert.equal(typeof m.selectedPageIds,'function');const pages=[{id:'p'},{id:'c',parentId:'p'},{id:'t',deleted:true}];assert.deepEqual(m.selectedPageIds(pages,new Set(['p','c','t']),false).sort(),['c','p']);assert.deepEqual(m.selectedPageIds(pages,new Set(['p','t']),true),['t']);});
