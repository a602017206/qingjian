import test from 'node:test';
import assert from 'node:assert/strict';
const {propertyOptionItems,nextOptionKey}=await import('../ui/web/property-options.mjs').catch(()=>({}));
const options=[{id:'a',label:'待开始'},{id:'b',label:'待评审'},{id:'c',label:'Alpha'}];

test('属性已有匹配项排在明确创建入口之前',()=>{
 assert.equal(typeof propertyOptionItems,'function','需要属性选项模型');
 const items=propertyOptionItems(options,' 待 ',true);
 assert.deepEqual(items.map(item=>[item.kind,item.id,item.label]),[['option','a','待开始'],['option','b','待评审'],['create','待','待']]);
 assert.equal(new Set(items.map(item=>item.key)).size,3);
});
test('完整匹配不重复创建，空查询和关联也不创建',()=>{
 assert.equal(typeof propertyOptionItems,'function');
 assert.deepEqual(propertyOptionItems(options,'alpha',true).map(item=>item.id),['c']);
 assert.equal(propertyOptionItems(options,'',true).length,3);
 assert.deepEqual(propertyOptionItems(options,'不存在',false),[]);
 assert.equal(propertyOptionItems(options,'新项目',true)[0].kind,'create');
});
test('同名关联保留独立标识，选项内容不被改写',()=>{
 assert.equal(typeof propertyOptionItems,'function');
 const source=[{id:'x',label:'同名'},{id:'y',label:'同名'}],before=structuredClone(source),items=propertyOptionItems(source,'同',false);
 assert.equal(items.length,2);assert.notEqual(items[0].key,items[1].key);assert.deepEqual(source,before);
});
test('活动项上下循环且安全处理空结果和已消失选项',()=>{
 assert.equal(typeof nextOptionKey,'function','需要活动项键盘导航');
 const items=[{key:'first'},{key:'second'},{key:'create'}];
 assert.equal(nextOptionKey(items,'first',1),'second');assert.equal(nextOptionKey(items,'first',-1),'create');
 assert.equal(nextOptionKey(items,'create',1),'first');assert.equal(nextOptionKey(items,'gone',1),'first');assert.equal(nextOptionKey(items,'gone',-1),'create');
 assert.equal(nextOptionKey([],'first',1),'');
});
