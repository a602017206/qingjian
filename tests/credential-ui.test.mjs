import test from 'node:test';
import assert from 'node:assert/strict';
import {syncSetupHTML,validateSyncValues} from '../ui/web/sync-ui.mjs';
test('同步引导提供记住认证与状态，记住选项进入请求',()=>{
 const html=syncSetupHTML({name:'笔记',remote:'https://gitee.com/example/notes.git',branch:'main'});
 assert.ok(html.includes('name="remember"'));
 assert.ok(html.includes('credential-status'));
 assert.equal(validateSyncValues({name:'笔记',remote:'https://gitee.com/u/n.git',username:'user',token:'test-only',remember:'on'}).remember,true);
 assert.equal(validateSyncValues({name:'笔记',remote:'https://gitee.com/u/n.git',remember:undefined}).remember,false);
});
