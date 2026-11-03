import test from 'node:test';
import assert from 'node:assert/strict';
const {RecordBlockSession,cloneRecordBlocks,hasRecordBodyConflict,resolveRecordBodyConflict}=await import('../ui/web/record-blocks.mjs').catch(()=>({}));
const fixture=()=>({id:'owner',database:{columns:[{id:'name',name:'名称',type:'text'}],rows:[{id:'a',values:{name:'甲'},body:'  旧正文\n\n末尾\n'},{id:'b',values:{name:'乙'},body:'第二条'}]}});
test('打开旧记录只建立编辑视图，不改原文或自动迁移',()=>{
 assert.equal(typeof RecordBlockSession,'function');const owner=fixture(),before=structuredClone(owner),session=new RecordBlockSession(),row=owner.database.rows[0],page=session.page(owner,row);
 assert.equal(page.blocks[0].text,row.body);assert.equal(session.page(owner,row),page);assert.notEqual(session.page(owner,owner.database.rows[1]).id,page.id);assert.equal(session.commit(owner,row),false);assert.deepEqual(owner,before);
});
test('块修改只写回当前行，兼容正文与基线一致',()=>{
 assert.equal(typeof RecordBlockSession,'function');const owner=fixture(),session=new RecordBlockSession(),row=owner.database.rows[0],page=session.page(owner,row),other=structuredClone(owner.database.rows[1]);
 page.blocks[0].text+='续写';assert.equal(session.commit(owner,row),true);assert.equal(row.blocks[0].text,'  旧正文\n\n末尾\n续写');assert.equal(row.blockBody,row.body);assert.deepEqual(owner.database.rows[1],other);
 page.blocks=[{id:'heading',type:'h2',text:'小标题'},...page.blocks];session.commit(owner,row);assert.match(row.body,/## 小标题/);
});
test('旧客户端修改纯文本时保留两份，属性保存不能静默覆盖',()=>{
 assert.equal(typeof RecordBlockSession,'function');const owner=fixture(),session=new RecordBlockSession(),row=owner.database.rows[0];session.page(owner,row).blocks[0].text='块正文';session.commit(owner,row);row.body='旧客户端新正文';assert.equal(hasRecordBodyConflict(row),true);assert.equal(session.page(owner,row).locked,true);assert.equal(session.commit(owner,row),false);assert.equal(row.body,'旧客户端新正文');
 const before=structuredClone(row.blocks);resolveRecordBodyConflict(row);assert.deepEqual(row.blocks.slice(0,before.length),before);assert.ok(row.blocks.some(block=>block.text==='旧客户端新正文'));assert.equal(hasRecordBodyConflict(row),false);
});
test('复制内容块重新分配所有父子与分栏标识',()=>{
 assert.equal(typeof cloneRecordBlocks,'function');const blocks=[{id:'g',type:'columns',text:''},{id:'c',type:'column',text:'',parentId:'g'},{id:'p',type:'paragraph',text:'原文',parentId:'c'}],copy=cloneRecordBlocks(blocks);
 assert.ok(copy.every(block=>!blocks.some(old=>old.id===block.id)));assert.equal(copy[1].parentId,copy[0].id);assert.equal(copy[2].parentId,copy[1].id);copy[2].text='副本';assert.equal(blocks[2].text,'原文');
});
test('父库锁定和行切换保留正文上下文',()=>{
 assert.equal(typeof RecordBlockSession,'function');const owner=fixture(),session=new RecordBlockSession(),a=session.page(owner,owner.database.rows[0]),b=session.page(owner,owner.database.rows[1]);owner.locked=true;assert.equal(a.locked,true);assert.equal(b.locked,true);a.blocks[0].text='不能写';assert.throws(()=>session.commit(owner,owner.database.rows[0]));assert.equal(owner.database.rows[0].blocks,undefined);
});

test('批量复制普通记录重分配块与嵌套标识并保留正文',async()=>{const {batchDatabase}=await import('../ui/web/database-batch.mjs');const source={id:'owner',database:{columns:[{id:'name',type:'text'}],rows:[{id:'row',values:{name:'原记录'},body:'正文',blockBody:'正文',blocks:[{id:'a',type:'toggle',text:'正文'},{id:'b',type:'paragraph',text:'子项',parentId:'a'}]}]}};const result=batchDatabase(source,['row'],{kind:'duplicate'}),copy=result.page.database.rows[1];assert.notEqual(copy.blocks[0].id,'a');assert.equal(copy.blocks[1].parentId,copy.blocks[0].id);assert.equal(source.database.rows.length,1);});
