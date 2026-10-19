import {t} from './i18n.mjs';
import {escapeHTML as e,byteSize} from './core.mjs';
import {UpdateChecker,releasesURL} from './updates.mjs';
const choices=[['windows','x86_64'],['macos','aarch64'],['macos','x86_64'],['linux','x86_64'],['android','aarch64'],['ios','aarch64'],['ios','aarch64','simulator']];
const osLabel=os=>t('updateOs'+os[0].toUpperCase()+os.slice(1));
const keyOf=parts=>parts.join(':');
const choiceLabel=([os,arch,sim])=>osLabel(os)+' · '+(arch==='aarch64'?t('updateArchArm'):t('updateArchX64'))+(sim?' · '+t('updateSimulator'):'');
export function createUpdateUI({api,native,openExternal,toast,showAbout}){
 let info=null,infoPromise=null,choice='',lastAttempt=0,timer,interval;
 const read=key=>{try{return localStorage.getItem('qingjian:update:'+key);}catch{return null;}};
 const remember=(key,value)=>{try{value===null?localStorage.removeItem('qingjian:update:'+key):localStorage.setItem('qingjian:update:'+key,value);}catch{}};
 let disabled=read('disabled')==='true',dismissed=read('dismissed');
 const enabled=()=>!disabled;
 const loadInfo=()=>infoPromise??=api('appInfo').then(value=>{info={...value,os:native()?value.os:'web',arch:native()?value.arch:'unknown'};redraw();return info;}).catch(error=>{infoPromise=null;throw error;});
 const runtimeInfo=async()=>{const context=await loadInfo();const selected=choices.find(c=>keyOf(c)===choice);return context.os==='web'&&selected?{...context,os:selected[0],arch:selected[1],simulator:!!selected[2]}:context;};
 const notification=document.createElement('aside');notification.id='update-notification';notification.setAttribute('role','status');notification.setAttribute('aria-live','polite');document.body.append(notification);
 const checker=new UpdateChecker({info:runtimeInfo,onChange:()=>redraw()});
 const btn=(label,action,attrs='')=>`<button type="button" class="button${attrs.includes('data-update-file=')?' primary':''}" data-update-action="${action}" ${attrs}>${e(label)}</button>`;
 function panelHTML(){
  const state=checker.state,context=info||{},busy=state.status==='checking';
  const message=state.status==='error'?t(state.error):busy?t('checkingUpdates'):state.newer?t('updateAvailable'):state.status==='current'?t('updateCurrent'):t('updateIdle');
  return `<section class="update-panel" data-status="${state.status}"><h3>${t('checkUpdates')}</h3><div class="update-version"><span>${t('currentVersion')}</span><strong>${e(context.version||'…')}</strong></div>${info?`<p class="form-hint">${e(osLabel(context.os))} · ${e(context.arch==='unknown'?t('updateUnknown'):context.arch)}</p>`:''}${context.os==='web'?`<label class="form-field">${t('updatePlatform')}<select id="update-platform" ${busy?'disabled':''}><option value="">${t('updateChoosePlatform')}</option>${choices.map(parts=>`<option value="${keyOf(parts)}" ${choice===keyOf(parts)?'selected':''}>${e(choiceLabel(parts))}</option>`).join('')}</select><small>${t('updatePreview')}</small></label>`:''}<p class="update-status ${state.status==='error'?'calculation-error':''}" role="status">${e(message)}</p>${state.newer?`<div class="update-version"><span>${t('latestVersion')}</span><strong>${e(state.version)}</strong></div>`:''}<div class="update-actions">${btn(busy?t('checkingUpdates'):t('checkUpdates'),'check',busy?'disabled':'')}${(state.downloads||[]).map(file=>btn((file.name.endsWith('.AppImage')?t('updateDownloadPortable'):file.name.endsWith('.deb')?t('updateDownloadDeb'):t('updateDownload'))+' · '+byteSize(file.size),'download',`data-update-file="${e(file.name)}"`)).join('')}${btn(t('updateNotes'),'notes')}</div>${state.newer&&!state.downloads.length?`<p class="form-hint">${t('updateNoAsset')}</p>`:''}${state.downloads?.length?`<p class="form-hint">${state.info.os==='ios'?t(state.info.simulator?'updateSimulator':'updateIos'):t('updateInstallHint')}</p>`:''}${state.notes?`<details class="update-notes"><summary>${t('updateNotes')}</summary><pre>${e(state.notes)}</pre></details>`:''}<label class="update-auto"><input id="update-auto" type="checkbox" ${enabled()?'checked':''}>${t('updateAuto')}</label><p class="form-hint">${t('updatePrivacy')}</p></section>`;
 }
 function redraw(){
  const target=document.querySelector('#update-panel-root');if(target)target.innerHTML=panelHTML();
  const state=checker.state,show=state.newer&&dismissed!==state.version;
  notification.hidden=!show;notification.innerHTML=show?`<div><strong>${t('updateAvailable')} · ${e(state.version)}</strong><span>${t('app')}</span></div>${btn(t('updateView'),'view')}${btn(t('updateLater'),'dismiss')}`:'';
 }
 async function check(manual=false){lastAttempt=Date.now();if(manual){dismissed=null;remember('dismissed',null);}await checker.check();}
 document.addEventListener('change',event=>{if(event.target.id==='update-platform'){choice=event.target.value;checker.state={status:'idle'};redraw();}if(event.target.id==='update-auto'){disabled=!event.target.checked;remember('disabled',String(disabled));if(!enabled())clearTimeout(timer);}});
 document.addEventListener('click',async event=>{const button=event.target.closest('[data-update-action]');if(!button||button.disabled)return;event.preventDefault();try{switch(button.dataset.updateAction){case 'check':await check(true);break;case 'download':{const download=checker.state.downloads?.find(file=>file.name===button.dataset.updateFile);if(!download)throw Error('updateDownloadFailed');await openExternal(download.url);break;}case 'notes':await openExternal(checker.state.pageURL||releasesURL);break;case 'view':await showAbout();break;case 'dismiss':dismissed=checker.state.version;remember('dismissed',dismissed);redraw();break;}}catch{toast(t('updateDownloadFailed'),true);}});
 function start(){loadInfo().catch(()=>{checker.state={status:'error',error:'updateInfoFailed'};redraw();});if(!native())return;
  timer=setTimeout(()=>{if(enabled())check();},1800);
  interval=setInterval(()=>{if(enabled())check();},3600000);
  document.addEventListener('visibilitychange',()=>{if(!document.hidden&&enabled()&&Date.now()-lastAttempt>=3600000)check();});
 }
 return {start,panelHTML,loadInfo,checker,stop:()=>{clearTimeout(timer);clearInterval(interval);}};
}
