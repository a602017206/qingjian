import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {spawn} from 'node:child_process';
import {mkdir} from 'node:fs/promises';
import path from 'node:path';
const addr='127.0.0.1:3446',base=`http://${addr}`,root=path.resolve('.test-data','credential-ui-'+Date.now());await mkdir(root,{recursive:true});await mkdir('test-results',{recursive:true});
const server=spawn(path.resolve('target/debug/server'),[],{env:{...process.env,QINGJIAN_DEV_ADDR:addr,QINGJIAN_DATA_DIR:root},stdio:'pipe'});let browser;
async function api(action,request={}){const r=await(await fetch(`${base}/api/${action}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request)})).json();if(r.error)throw new Error(r.error);return r.data;}
try{
 for(let i=0;i<100;i++){try{if((await fetch(base)).ok)break;}catch{}await new Promise(r=>setTimeout(r,100));}
 browser=await chromium.launch({headless:true,...(process.platform==='darwin'?{channel:'chrome'}:{})});
 for(const phone of [false,true]){
  const w=await api('createWorkspace',{name:phone?'手机匿名测试':'桌面匿名测试'}),vault=new Map(),requests=[];
  const context=await browser.newContext({viewport:phone?{width:390,height:844}:{width:1360,height:960},isMobile:phone,hasTouch:phone});
  await context.exposeBinding('testNativeDispatch',async(_,args)=>{
   const {action,request}=args;if(action==='credentialCapabilities')return {available:true};
   const id=request.workspaceId,old=vault.get(id);
   if(action==='credentialStatus')return {available:true,saved:!!old,session:!!old,username:old?.username||''};
   if(action==='credentials'){requests.push({...request});const auth={username:request.username||old?.username,token:request.token||old?.token};if(!auth.username||!auth.token)throw new Error('匿名测试缺少认证');if(request.remember)vault.set(id,auth);else vault.delete(id);return {available:true,saved:!!request.remember,session:true,username:auth.username};}
   if(action==='forgetCredentials'){vault.delete(id);return {available:true,saved:false,session:false,username:''};}
   if(action==='sync')return {status:'success',message:'匿名界面测试同步完成'};
   return api(action,request);
  });
  await context.addInitScript(()=>{window.isTauri=true;window.__TAURI_INTERNALS__={invoke:async(cmd,args)=>{if(cmd!=='dispatch')throw '匿名测试不调用原生插件';return window.testNativeDispatch(args);}};});
  const page=await context.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));await page.route('https://api.github.com/**',route=>route.fulfill({status:503,body:'匿名测试不访问更新服务'}));await page.goto(`${base}/?workspace=${w.id}`);await page.waitForSelector('.home-heading');await page.locator('.topbar [data-action=sync]').click();
  const form=page.locator('#sync-setup-form');await form.locator('[name=remember]:enabled').waitFor();assert.equal(await form.locator('[name=remember]').isChecked(),true);
  await form.locator('[name=remote]').fill('https://gitee.com/anonymous/notes.git');await form.locator('[name=username]').fill('anonymous-user');await form.locator('[name=token]').fill('anonymous-test-secret');await form.locator('[type=submit]').click();await form.waitFor({state:'detached'});assert.equal(requests[0].remember,true);
  await page.reload();await page.waitForSelector('.home-heading');
  // 重新进入同步设置，模拟原生凭据库重启后恢复；真实钥匙串另由原生测试验证。
  if(phone&&!await page.locator('.sidebar').evaluate(el=>el.classList.contains('visible')))await page.locator('.topbar [data-action=sidebar]').click();await page.locator('[data-action=settings]').first().click();await page.locator('[data-action=syncSetup]').click();await form.locator('.credential-status').filter({hasText:'已安全记住'}).waitFor();assert.equal(await form.locator('[name=username]').inputValue(),'anonymous-user');assert.equal(await form.locator('[name=token]').inputValue(),'');
  await page.screenshot({path:`test-results/${phone?'手机':'桌面'}记住工作区认证.png`,animations:'disabled'});await form.locator('[type=submit]').click();await form.waitFor({state:'detached'});assert.equal(requests[1].token,'');assert.equal(vault.get(w.id).token,'anonymous-test-secret');
  if(phone&&!await page.locator('.sidebar').evaluate(el=>el.classList.contains('visible')))await page.locator('.topbar [data-action=sidebar]').click();await page.locator('[data-action=settings]').first().click();const auth=page.locator('#auth-form');await auth.locator('.credential-forget:not([hidden])').waitFor();await auth.locator('.credential-forget').click();await auth.locator('.credential-status').filter({hasText:'已清除'}).waitFor();assert.equal(vault.has(w.id),false);assert.equal(await auth.locator('[name=username]').inputValue(),'');assert.ok(!(await page.evaluate(()=>JSON.stringify(localStorage))).includes('anonymous-test-secret'));assert.deepEqual(errors,[]);await context.close();
 }
 console.log('桌面和手机认证界面：默认记住、恢复用户名、隐藏令牌、空令牌沿用与忘记认证通过（模拟原生安全存储）');
}finally{await browser?.close();server.kill('SIGTERM');}
