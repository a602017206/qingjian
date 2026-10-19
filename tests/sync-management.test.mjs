import test from 'node:test';
import assert from 'node:assert/strict';
import {syncConflictHTML,workspaceManagementHTML} from '../ui/web/sync-management.mjs';
test('同步冲突显示双方摘要，独立历史没有预选覆盖策略',()=>{
 const html=syncConflictHTML({unrelated:true,localPages:1,remotePages:2,localCommit:'a'.repeat(40),remoteCommit:'b'.repeat(40),conflicts:[{path:'pages/one.json',local:{title:'<本机>',blocks:[{text:'本机内容'}]},remote:{title:'远程标题',blocks:[{text:'远程内容'}]}}]});
 assert.ok(html.includes('&lt;本机&gt;'));assert.ok(html.includes('远程内容'));assert.ok(html.includes('name="strategy"'));assert.ok(!html.includes('checked'));assert.ok(html.includes('data-conflict-path="pages/one.json"'));
});
test('工作区移除入口和已移除目录同时可达',()=>{
 const html=workspaceManagementHTML([{id:'one',name:'当前工作区',remote:'',}], [{id:'old',workspace:{name:'旧工作区'},bytes:1024}], 'one');
 assert.ok(html.includes('data-action="removeWorkspace"'));assert.ok(html.includes('data-action="restoreWorkspace"'));assert.ok(html.includes('data-action="purgeWorkspace"'));assert.ok(html.includes('data-action="syncBackups"'));
});
