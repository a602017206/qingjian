import test from 'node:test';
import assert from 'node:assert/strict';
const module=await import('../ui/web/import-repair.mjs').catch(()=>({}));
const repair=pages=>{assert.equal(typeof module.importRepairSuggestions,'function','缺少已有数据整理功能');return module.importRepairSuggestions(pages);};
const db={id:'shows',title:'影视剧',database:{columns:[{id:'name',name:'电影名称',type:'text'},{id:'status',name:'是否观看',type:'text'},{id:'types',name:'类型',type:'text'},{id:'date',name:'观看时间',type:'text'},{id:'notes',name:'备注',type:'text'}],rows:[{id:'r',values:{name:'牧神记',status:'观看中',types:'动漫, 电影',date:'2025/10/15',notes:'观看中'}}]}};
test('整理明确字段类型并保留原值与用户已配置的属性',()=>{
 const before=JSON.stringify(db),result=repair([db]);const change=result.fields[0];assert.equal(change.pageId,'shows');assert.equal(change.page.database.columns[1].type,'status');assert.equal(change.page.database.columns[2].type,'multiselect');assert.equal(change.page.database.rows[0].values.types,'["动漫","电影"]');assert.equal(change.page.database.rows[0].sourceValues.types,'动漫, 电影');assert.equal(change.page.database.rows[0].values.date,'2025-10-15');assert.equal(change.page.database.columns[4].type,'text');assert.equal(JSON.stringify(db),before);
 const configured=structuredClone(db);configured.database.columns[1].type='status';configured.database.columns[1].optionColors={'观看中':'purple'};assert.equal(repair([configured]).fields[0].page.database.columns[1].optionColors['观看中'],'purple');
});
test('子页面唯一匹配才建议归属，已有父级、重复标题与循环不改动',()=>{
 const child={id:'child',title:'牧神记',parentId:'',blocks:[{text:'正文'}]},parented={...child,id:'parented',parentId:'other'};
 assert.deepEqual(repair([db,child,parented]).parents.map(p=>[p.pageId,p.parentId]),[['child','shows']]);
 assert.deepEqual(repair([db,child,{...child,id:'duplicate'}]).parents,[]);
 assert.deepEqual(repair([db,{...db,id:'other'},child]).parents,[]);
 assert.deepEqual(repair([{...db,parentId:'child'},child]).parents,[]);
});
test('无效日期保留文本，不把正文或网址推断成选项',()=>{
 const p=structuredClone(db);p.database.rows[0].values.date='2025/02/31';p.database.columns.push({id:'url',name:'网址',type:'text'});p.database.rows[0].values.url='https://example.com';const result=repair([p]).fields[0].page;assert.equal(result.database.columns[3].type,'text');assert.equal(result.database.rows[0].values.date,'2025/02/31');
});
test('指定当前数据库后只整理该数据库，多个导出视图不会阻断明确的归属选择',()=>{
 const other={...structuredClone(db),id:'all',title:'影视剧完整字段'},child={id:'child',title:'牧神记',parentId:''};
 const result=module.importRepairSuggestions([db,other,child],'shows');assert.equal(result.fields.length,1);assert.equal(result.fields[0].pageId,'shows');assert.equal(result.parents[0]?.parentId,'shows');
});
