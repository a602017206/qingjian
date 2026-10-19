import test from 'node:test';import assert from 'node:assert/strict';
import {syncSetupHTML,syncProvider,validateSyncValues} from '../ui/web/sync-ui.mjs';
test('国内仓库入口、授权页面和真实用户名说明可达',()=>{const html=syncSetupHTML({name:'笔记',remote:'https://gitee.com/example/notes.git',branch:'main'});assert.ok(html.includes('https://gitee.com/projects/new'));assert.ok(html.includes('https://gitee.com/profile/personal_access_tokens'));assert.ok(html.includes('id="sync-provider"'));assert.ok(html.includes('Gitee'));});
test('站点按完整主机识别，国内仓库令牌不能使用空用户名',()=>{
 assert.equal(syncProvider('https://gitee.com/u/notes.git'),'gitee');assert.equal(syncProvider('https://gitee.com.evil.example/u/n.git'),'custom');
 assert.throws(()=>validateSyncValues({name:'笔记',remote:'https://gitee.com/u/n.git',token:'anonymous-token'}),/用户名/);
 assert.equal(validateSyncValues({name:'笔记',remote:'https://gitee.com/u/n.git',token:'anonymous-token',username:'actual-user',branch:'master'}).branch,'master');
 assert.throws(()=>validateSyncValues({name:'笔记',remote:'https://user:secret@gitee.com/u/n.git',username:'actual-user'}));
});
