import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {spawn} from 'node:child_process';
import {mkdir} from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3449',base=`http://${addr}`,server=spawn(path.resolve('target/debug/server'),[],{env:{...process.env,QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:path.resolve('.test-data',`editor-stability-${Date.now()}`)},stdio:'pipe'});let browser,workspaceId;
const api=async(action,data={})=>{const r=await(await fetch(base+'/api/'+action,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({workspaceId,...data})})).json();if(r.error)throw new Error(r.error);return r.data;};
try{
 await mkdir('test-results',{recursive:true});for(let n=0;n<100;n++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 workspaceId=(await api('createWorkspace',{name:'编辑排版匿名验证'})).id;
 const note=await api('save',{page:{title:'多行内容排版',blocks:[...Array.from({length:12},(_,i)=>({id:'p'+i,type:'paragraph',text:'上方正文 '+i})),{id:'a',type:'paragraph',text:'电脑\n  远程桌面\n    测试地址\n\n末行\n'},{id:'b',type:'paragraph',text:'包棋\n  安全访问系统\n    演示条目\n末行'},{id:'c',type:'paragraph',text:'后续正文'},{id:'toggle',type:'toggle',text:'嵌套父标题'},{id:'child',parentId:'toggle',type:'paragraph',text:'子内容\n  子内容第二行'},...Array.from({length:16},(_,i)=>({id:'tail'+i,type:'paragraph',text:'末尾正文 '+i}))]}});
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});const errors=[];
 for(const width of [1440,390]){
  const current=(await api('pages')).find(p=>p.id===note.id);await api('save',{page:{...current,blocks:note.blocks}});
  const page=await browser.newPage({viewport:{width,height:1000}});page.on('pageerror',error=>errors.push(error.message));await page.goto(`${base}/?workspace=${workspaceId}&page=${note.id}`);await page.waitForSelector('[data-edit=a]');const edit=id=>page.locator(`[data-edit="${id}"]`),row=id=>page.locator(`[data-block-id="${id}"]`);
  await edit('a').scrollIntoViewIfNeeded();await page.locator('.main-scroll').evaluate(el=>{el.style.scrollBehavior='auto';el.scrollTop+=document.querySelector('[data-edit=a]').getBoundingClientRect().top-220;});
  const beforeA=await row('a').boundingBox(),beforeB=await row('b').boundingBox(),scroll=await page.locator('.main-scroll').evaluate(el=>el.scrollTop);
  await edit('a').click();await page.waitForTimeout(180);const activeB=await row('b').boundingBox();assert.ok(Math.abs(activeB.y-beforeB.y)<=2,`进入编辑不能撑高正文：${activeB.y-beforeB.y}`);assert.equal(await page.locator('.main-scroll').evaluate(el=>el.scrollTop),scroll);
  const text=await page.locator('[data-input-block=a]').inputValue();assert.equal(text,note.blocks.find(b=>b.id==='a').text);const activeA=await row('a').boundingBox();assert.ok(Math.abs(activeA.height-beforeA.height)<=2,'普通多行文本编辑前后等高');
  await edit('b').click();await page.waitForTimeout(180);assert.equal(await page.locator('.block-input').count(),1,'只保留当前编辑框');console.log('切换排版',JSON.stringify({width,beforeA,beforeB,activeA,afterA:await row('a').boundingBox(),afterB:await row('b').boundingBox(),scroll,afterScroll:await page.locator('.main-scroll').evaluate(el=>el.scrollTop)}));assert.ok(Math.abs((await row('b').boundingBox()).y-beforeB.y)<=2,'切换块保持位置');
  // 再次进入前一块，预览保留每行开头的空格与空行。
  assert.equal(await edit('a').innerText(),text);await edit('a').click();await page.waitForTimeout(160);assert.ok(Math.abs((await row('b').boundingBox()).y-beforeB.y)<=2);
  const tools=page.locator('.inline-tools:visible');assert.equal(await tools.count(),1);assert.ok(await tools.evaluate(el=>{const r=el.getBoundingClientRect();return r.x>=0&&r.right<=innerWidth&&r.y>=0&&r.bottom<=innerHeight;}));
  await page.locator('[data-input-block=a]').evaluate(el=>el.setSelectionRange(0,2));await tools.locator('[data-format=bold]').click();assert.match(await page.locator('[data-input-block=a]').inputValue(),/^\*\*电脑\*\*/);assert.equal(await tools.isVisible(),true);
  await page.screenshot({path:`test-results/${width===390?'手机':'桌面'}编辑排版稳定.png`,animations:'disabled'});await page.close();
 }
 assert.deepEqual(errors,[]);console.log('多行缩进空行、进入退出编辑高度、连续切换滚动、单编辑框、浮动格式工具和手机边界通过');
}finally{await browser?.close();server.kill('SIGTERM');}
