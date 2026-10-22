import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {spawn} from 'node:child_process';
import path from 'node:path';

const addr='127.0.0.1:3450',base=`http://${addr}`;
const server=spawn(path.resolve(`target/debug/server${process.platform==='win32'?'.exe':''}`),[],{env:{...process.env,QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:path.resolve('.test-data',`editor-ime-${Date.now()}`)},stdio:'pipe'});
let browser,workspaceId;
const api=async(action,data={})=>{const result=await(await fetch(`${base}/api/${action}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({workspaceId,...data})})).json();if(result.error)throw Error(result.error);return result.data;};
const failures=[];
async function check(label,run){try{await run();console.log('通过：'+label);}catch(error){failures.push(label+'：'+error.message);console.error('失败：'+label+'：'+error.message);}}
// 明确测试浏览器事件边界；这不代替真实系统输入法验收。
const composingInput=(input,value)=>input.evaluate((el,value)=>{el.focus();el.dataset.imeProbe='original';el.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true}));el.value=value;el.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertCompositionText',data:value,isComposing:true}));},value);
const finishInput=input=>input.evaluate(el=>{el.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true,data:el.value}));el.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertText',data:el.value}));});
const imeKey=(input,key,legacy=false)=>input.evaluate((el,{key,legacy})=>el.dispatchEvent(new KeyboardEvent('keydown',{key,bubbles:true,cancelable:true,isComposing:!legacy,keyCode:legacy?229:0})),{key,legacy});
try{
 for(let n=0;n<100;n++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(resolve=>setTimeout(resolve,100));}
 workspaceId=(await api('createWorkspace',{name:'匿名输入法边界验证'})).id;
 const note=await api('save',{page:{title:'输入法正文',blocks:[{id:'text',type:'paragraph',text:'原文'}]}});
 const database=await api('save',{page:{title:'输入法记录',blocks:[],database:{columns:[{id:'name',name:'名称',type:'text'},{id:'status',name:'状态',type:'status',options:['待开始','已完成']}],rows:[{id:'raw',values:{name:'匿名记录',status:'待开始'},body:'匿名正文'},{id:'linked',pageId:note.id,values:{name:'独立子页',status:'待开始'}}]}}});
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 for(const width of [1440,390]){
  const current=(await api('pages')).find(p=>p.id===note.id);await api('save',{page:{...current,blocks:note.blocks}});
  const page=await browser.newPage({viewport:{width,height:900}}),errors=[];page.setDefaultTimeout(5000);page.on('pageerror',error=>errors.push(error.message));
  const open=async id=>{await page.goto(`${base}/?workspace=${workspaceId}&page=${id}`);await page.waitForSelector('#page-title:not([readonly])');};
  // 导航恢复草稿期间正文会暂时只读；点击返回不代表侧览已可编辑。
  const openRow=async id=>{await page.locator(`[data-action=${width<600?'openMobileRow':'openRow'}][data-id=${id}]`).click();await page.waitForSelector(id==='linked'?'.record-peek #page-title:not([readonly])':'.record-peek #record-body:not([readonly])');};
  for(const legacy of [false,true]){
   const mode=legacy?'兼容码229':'组词标志';
   await check(`${width} ${mode}属性回车不提交`,async()=>{
    await open(database.id);await page.locator('[data-action=selectValues][data-id=raw][data-column=status]').click();const input=page.locator('#selection-search');await composingInput(input,'新状态');await imeKey(input,'Enter',legacy);
    assert.equal(await input.count(),1,'组词确认不应关闭属性弹层');assert.equal(await page.locator('[data-option-value="新状态"]').count(),0,'组词确认不应创建选项');await finishInput(input);await input.press('Enter');await page.waitForSelector('#selection-search',{state:'detached'});await page.keyboard.press(process.platform==='darwin'?'Meta+s':'Control+s');await page.waitForFunction(()=>document.querySelector('#save-state')?.textContent.includes('已保存'));
    const saved=(await api('pages')).find(p=>p.id===database.id);assert.equal(saved.database.rows[0].values.status,'新状态','组词完成后正常回车仍能提交');
    // 每个场景恢复初始属性，避免前一场景掩盖误创建行为。
    saved.database.columns.find(c=>c.id==='status').options=['待开始','已完成'];saved.database.rows[0].values.status='待开始';await api('save',{page:saved});
   });
   await check(`${width} ${mode}侧览取消候选不关闭`,async()=>{
    await open(database.id);await openRow('raw');const input=page.locator('#record-body');await composingInput(input,'保留组词正文');await imeKey(input,'Escape',legacy);assert.equal(await input.count(),1,'取消候选不应关闭记录侧览');assert.equal(await input.evaluate(el=>el===document.activeElement),true);await finishInput(input);await input.press('Escape');await page.waitForSelector('.record-peek',{state:'detached'});assert.equal((await api('pages')).find(p=>p.id===database.id).database.rows[0].body,'保留组词正文');
   });
   await check(`${width} ${mode}独立子页回车不拆块`,async()=>{
    await open(database.id);await openRow('linked');await page.locator('.record-peek [data-edit=text]').click();const input=page.locator('[data-input-block=text]');await composingInput(input,'中文正文');await imeKey(input,'Enter',legacy);assert.equal(await page.locator('.record-peek .block').count(),1,'组词回车不应拆分正文');await finishInput(input);
   });
  }
  await check(`${width}菜单搜索保留输入框和候选`,async()=>{
   await open(note.id);await page.locator('[data-action=addBlock]').click();const input=page.locator('#block-command-search');await composingInput(input,'折叠');assert.equal(await input.getAttribute('data-ime-probe'),'original','输入时不能替换搜索框');assert.equal(await input.evaluate(el=>el===document.activeElement),true,'输入焦点保持');await imeKey(input,'Enter');assert.equal(await input.count(),1,'组词确认不能执行命令');await finishInput(input);assert.equal(await input.getAttribute('data-ime-probe'),'original','完成组词后仍保留输入框');assert.equal(await page.locator('.block-command-list [data-type=toggle]').count(),1);await input.press('Enter');await page.waitForSelector('.block-popover',{state:'detached'});assert.equal(await page.locator('.block-toggle').count(),1,'正常回车可以插入折叠块');
  });
  assert.deepEqual(errors,[]);await page.close();
 }
 assert.deepEqual(failures,[],'输入法保护回归');console.log('桌面与手机输入法事件边界回归通过；真实输入法仍需设备验收');
}finally{await browser?.close();server.kill('SIGTERM');}
