import test from 'node:test';
import assert from 'node:assert/strict';
const {bindSearchInput}=await import('../ui/web/search-input.mjs').catch(()=>({}));
const pause=()=>new Promise(resolve=>setTimeout(resolve,25));
function setup(){const input={value:'',isConnected:true},queries=[],renders=[];assert.equal(typeof bindSearchInput,'function','需要不重建节点的组词搜索绑定');bindSearchInput(input,{delay:5,update:value=>queries.push(value),refresh:()=>renders.push(input.value)});return {input,queries,renders};}

test('中文组词结束前不更新查询或结果，最终输入只刷新一次',async()=>{
 const {input,queries,renders}=setup();input.oncompositionstart();input.value='zhong';input.oninput({isComposing:true});await pause();assert.deepEqual(queries,[]);assert.deepEqual(renders,[]);
 input.value='中文';input.oncompositionend();input.oninput({isComposing:false});await pause();assert.deepEqual(queries,['中文']);assert.deepEqual(renders,['中文']);
});
test('连续普通输入合并结果刷新且保留输入框属性',async()=>{
 const {input,queries,renders}=setup();input.selectionStart=1;input.selectionEnd=2;input.value='a';input.oninput({});input.value='ab';input.oninput({});await pause();
 assert.deepEqual(queries,['a','ab']);assert.deepEqual(renders,['ab']);assert.equal(input.selectionStart,1);assert.equal(input.selectionEnd,2);
});
test('导航移除输入框后旧搜索任务不能刷新新页面',async()=>{
 const {input,renders}=setup();input.value='旧页';input.oninput({});input.isConnected=false;await pause();assert.deepEqual(renders,[]);
});
test('开始组词时取消先前待执行的搜索',async()=>{
 const {input,renders}=setup();input.value='先前';input.oninput({});input.oncompositionstart();await pause();assert.deepEqual(renders,[]);input.value='最终';input.oncompositionend();await pause();assert.deepEqual(renders,['最终']);
});
