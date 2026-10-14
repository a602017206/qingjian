import test from 'node:test';
import assert from 'node:assert/strict';
import { linkedRecord,visibleRecordBlocks,renderRecordProperties,recordValueDraft } from '../ui/web/record-page.mjs';
const child={id:'child',parentId:'parent',title:'子页',tags:[],importSource:'库/子页.html',importProperties:[{name:'优先级',raw:'-1',type:'number'},{name:'状态',raw:'观看中',type:'status'}],blocks:[{id:'snapshot',type:'paragraph',text:'优先级: -1\n状态: 观看中'},{id:'body',type:'paragraph',text:'自己的观影感想'}]};
const parent={id:'parent',revision:4,database:{columns:[{id:'name',name:'名称',type:'text'},{id:'status',name:'状态',type:'status',options:['观看中','已完成'],optionColors:{'观看中':'blue','已完成':'green'}},{id:'priority',name:'优先级',type:'number'},{id:'date',name:'上映时间',type:'date'}],rows:[{id:'row',pageId:'child',values:{name:'子页',status:'观看中',priority:'-1',date:''},sourceValues:{name:'子页',status:'观看中',priority:'-1',date:''}}]}};
test('子页属性使用唯一父数据库关联，歧义和错误父级不猜测',()=>{
 assert.equal(linkedRecord(child,[parent])?.row.id,'row');
 assert.equal(linkedRecord({...child,parentId:'other'},[parent]),null);
 assert.equal(linkedRecord(child,[{...parent,deleted:true}]),null);
 assert.equal(linkedRecord(child,[{...parent,database:{...parent.database,rows:[parent.database.rows[0],{...parent.database.rows[0],id:'duplicate'}]}}]),null);
});
test('属性显示类型、原色与空白，不重复显示标题，锁定父页不允许编辑',()=>{
 const html=renderRecordProperties(child,[parent]);assert.match(html,/option-blue/);assert.match(html,/type="number"/);assert.match(html,/type="date"/);assert.match(html,/空白/);assert.ok(!html.includes('aria-label="名称"'));
 assert.ok(html.indexOf('优先级')<html.indexOf('状态'));
 const locked=renderRecordProperties(child,[{...parent,locked:true}]);assert.match(locked,/disabled/);
});
test('只有完整匹配的导入属性快照隐藏，真实正文和修改过的文字保留',()=>{
 const context=linkedRecord(child,[parent]);assert.deepEqual(visibleRecordBlocks(child,context).map(b=>b.id),['body']);
 assert.equal(visibleRecordBlocks({...child,importSource:''},context).length,2);
 assert.equal(visibleRecordBlocks({...child,blocks:[{...child.blocks[0],text:'状态: 不同内容'}]},context).length,1);
 assert.equal(visibleRecordBlocks({...child,blocks:[{...child.blocks[0],text:'状态: 观看中\n这是我的补充'}]},context).length,1);
 const edited={...parent,database:{...parent.database,rows:[{...parent.database.rows[0],values:{...parent.database.rows[0].values,status:'已完成'}}]}};
 assert.deepEqual(visibleRecordBlocks(child,linkedRecord(child,[edited])).map(b=>b.id),['body']);
});
test('属性草稿只修改关联父页当前值，不覆盖来源、其他记录及子页正文',()=>{
 const draft=recordValueDraft(child,[parent],'row','status','已完成');
 assert.equal(draft.database.rows[0].values.status,'已完成');assert.equal(draft.database.rows[0].sourceValues.status,'观看中');assert.equal(parent.database.rows[0].values.status,'观看中');assert.equal(draft.revision,4);
 assert.throws(()=>recordValueDraft(child,[{...parent,locked:true}],'row','status','已完成'));
 assert.throws(()=>recordValueDraft(child,[parent],'row','missing','值'));
});

test('属性保存失败保留草稿，保存期间的后续输入按新版本继续保存',async()=>{
 const {RecordDraftQueue}=await import('../ui/web/record-page.mjs');
 let fail=true,release,saves=[];
 const queue=new RecordDraftQueue({save:async(workspace,page)=>{if(fail)throw new Error('模拟保存失败');if(saves.length===0)await new Promise(resolve=>release=resolve);saves.push({workspace,page});return {...page,revision:page.revision+1};}});
 queue.edit('workspace',child,[parent],'row','priority','2');
 await assert.rejects(queue.flush(),/模拟保存失败/);assert.equal(queue.dirty,true);assert.equal(queue.page.database.rows[0].values.priority,'2');
 fail=false;const saving=queue.flush();await Promise.resolve();
 queue.edit('workspace',child,[parent],'row','priority','3');release();await saving;
 assert.equal(saves.length,2);assert.equal(saves[1].page.revision,5);assert.equal(saves[1].page.database.rows[0].values.priority,'3');assert.equal(queue.dirty,false);
 assert.ok(saves.every(s=>s.workspace==='workspace'));
});

test('并发触发保存共用同一个队列，不重复写入旧版本',async()=>{
 const {RecordDraftQueue}=await import('../ui/web/record-page.mjs');let release;const saved=[];
 const queue=new RecordDraftQueue({save:async(workspace,page)=>{if(!saved.length)await new Promise(resolve=>release=resolve);saved.push(page);return {...page,revision:page.revision+1};}});
 queue.edit('workspace',child,[parent],'row','priority','2');const first=queue.flush();await Promise.resolve();queue.edit('workspace',child,[parent],'row','priority','3');const second=queue.flush();release();await Promise.all([first,second]);assert.equal(saved.length,2);
});
test('导入属性快照被用户删减后，不能把剩下的手写内容继续隐藏',()=>{
 const blocks=visibleRecordBlocks({...child,blocks:[{id:'manual',type:'paragraph',text:'状态: 观看中'}]},linkedRecord(child,[parent]));assert.equal(blocks.length,1);
});
