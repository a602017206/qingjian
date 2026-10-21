import {AutoSyncController,reconcileSyncDraft,syncPageContent} from './auto-sync.mjs';
import {watchMutations,setRequestGate,setSaveRecovery,dataMutations,callAPI} from './api.mjs';
import {mergeSavedMetadata} from './property-edit.mjs';
import {escapeHTML as e} from './text.mjs';
import {t} from './i18n.mjs';
import {errorMessage} from './errors.mjs';
export function createAutoSyncUI({state,recordDraft,workspace,flush,render,renderSaveState,modal,closeModal,toast,showSyncConflict,manualSync,applyConfig}){
 let network=null,conflicts=[],remoteConflicts=new Map(),composing=false;
 const context=()=>({id:workspace()?.id,remote:workspace()?.remote,enabled:workspace()?.autoSync!==false,visible:!document.hidden,online:navigator.onLine!==false,busy:state.busy||!!document.querySelector('.modal,.property-popover')||composing||!!state.saving||!!recordDraft.saving});
 const conflictKey=()=>context().id+'|'+context().remote;
 const controller=new AutoSyncController({context,run:async id=>{if(navigator.locks?.request)return navigator.locks.request('qingjian:auto-sync:'+id,{ifAvailable:true},lock=>lock?run(id):{status:'deferred'});return run(id);},onChange:redraw});
 function label(){const c=context();if(conflicts.length)return t('autoSyncNeedsChoice');return !c.remote?'':!c.enabled?t('autoSyncDisabled'):t({idle:'autoSyncWaiting',pending:'autoSyncWaiting',syncing:'autoSyncWorking',current:'autoSyncCurrent',error:'autoSyncRetryLater',offline:'autoSyncOffline',conflict:'autoSyncNeedsChoice',disabled:'autoSyncDisabled'}[controller.entry().status]||'autoSyncWaiting');}
 function statusHTML(){return `<button type="button" class="auto-sync-status" data-action="autoSyncStatus" data-status="${controller.entry().status}" ${!workspace()?.remote&&!conflicts.length?'hidden':''} title="${e(controller.entry().error||label())}">${e(label())}</button>`;}
 function redraw(){for(const el of document.querySelectorAll('.auto-sync-status')){el.hidden=!workspace()?.remote&&!conflicts.length;el.textContent=label();el.dataset.status=controller.entry().status;el.title=controller.entry().error||label();}}
 setRequestGate(async action=>{if(network&&(dataMutations.has(action)||['sync','resolveSync','activate','updateWorkspace','removeWorkspace','autoSyncSettings'].includes(action)))await network;});
 setSaveRecovery(async(request,error)=>{
  if(request.workspaceId!==context().id||network)return null;
  let pages;try{pages=await callAPI('pages',{workspaceId:request.workspaceId});}catch{return null;}
  const remote=pages.find(p=>p.id===request.page.id)||null;if(remote?.revision===request.page.revision)return null;
  const old=state.pages.find(p=>p.id===request.page.id),base=old?.revision===request.page.revision?old:null;
  const decision=reconcileSyncDraft(base,request.page,remote);
  if(decision.page&&(decision.kind==='local'||(decision.kind==='clean'&&syncPageContent(request.page)===syncPageContent(remote)))){for(const live of [state.page,recordDraft.page])if(live?.id===request.page.id&&live.revision===request.page.revision)mergeSavedMetadata(live,decision.page);return {...request,page:decision.page};}
  const source=state.page?.id===request.page.id?'page':recordDraft.page?.id===request.page.id?'record':'external';
  const live=source==='page'?state.page:source==='record'?recordDraft.page:request.page;
  if(!conflicts.some(item=>item.draft.id===live.id)){const draft=structuredClone(live);const sourceCommit=state.status?.commit;const preview=await callAPI('syncDraftPreview',{workspaceId:request.workspaceId,draft,sourceCommit});if(request.workspaceId!==context().id)return null;conflicts.push({source,draft,preview,sourceCommit});}
  controller.pause();return null;
 });
 watchMutations(({action,workspaceId})=>{if(action==='credentials'&&workspaceId===context().id)controller.resume();else controller.request('edit',workspaceId);});
 function rememberDraft(){if(state.dirty&&state.page){try{localStorage.setItem(`qingjian:recovery:${context().id}:${state.page.id}`,JSON.stringify(state.page));}catch{}}if(recordDraft.page)recordDraft.onChange();}
 function applyDraft(live,base,remote,dirty,source){
  if(!live)return false;
  const decision=reconcileSyncDraft(base,dirty?live:base,remote);
  if(decision.kind==='conflict'){conflicts.push({source,draft:structuredClone(live),preview:null});return false;}
  if(decision.kind==='local'){mergeSavedMetadata(live,decision.page);return false;}
  if(source==='page'){state.dirty=false;try{localStorage.removeItem(`qingjian:recovery:${context().id}:${live.id}`);}catch{}if(remote&&!remote.deleted){if(syncPageContent(live)===syncPageContent(remote))mergeSavedMetadata(live,remote);else state.page=structuredClone(remote);}else{state.page=null;state.route='home';state.recordPeek=null;}}
  else{recordDraft.dirty=false;if(remote&&syncPageContent(live)===syncPageContent(remote))mergeSavedMetadata(live,remote);else recordDraft.page=remote?structuredClone(remote):null;recordDraft.onChange();}
  return syncPageContent(base)!==syncPageContent(remote);
 }
 async function run(id){if(id!==context().id)return {status:'deferred'};await flush();if(context().busy||id!==context().id||!context().enabled)return {status:'deferred'};
  let unlock;network=new Promise(resolve=>unlock=resolve);const before=structuredClone(state.pages),pageId=state.page?.id;let result,rerender=false;
  let syncError,sourceCommit;try{
   sourceCommit=(await callAPI('commit',{workspaceId:id})).commit;
   try{result=await callAPI('sync',{workspaceId:id});}catch(error){syncError=error;}
   const pages=await callAPI('pages',{workspaceId:id});const extras=await Promise.allSettled(['stats','status'].map(action=>callAPI(action,{workspaceId:id})));const stats=extras[0].status==='fulfilled'?extras[0].value:state.stats,status=extras[1].status==='fulfilled'?extras[1].value:state.status;
   if(composing)await new Promise(resolve=>document.addEventListener('compositionend',resolve,{once:true}));
   if(id!==context().id)return {status:'deferred'};
   const focus=document.activeElement,block=focus?.dataset?.inputBlock,caret=focus?.selectionStart,end=focus?.selectionEnd;
   if(state.page&&state.page.id===pageId)rerender=applyDraft(state.page,before.find(p=>p.id===pageId),pages.find(p=>p.id===pageId)||null,state.dirty,'page')||rerender;
   if(recordDraft.page)rerender=applyDraft(recordDraft.page,before.find(p=>p.id===recordDraft.page.id),pages.find(p=>p.id===recordDraft.page.id)||null,recordDraft.dirty,'record')||rerender;
   const listChanged=JSON.stringify(state.pages)!==JSON.stringify(pages);state.pages=pages;state.stats=stats;state.status=status;rememberDraft();
   for(const conflict of conflicts){conflict.sourceCommit=sourceCommit;conflict.preview=await callAPI('syncDraftPreview',{workspaceId:id,draft:conflict.draft,sourceCommit});}
   if(conflicts.length){controller.pause();result={status:'conflict'};}
   else if(result?.status==='conflict')remoteConflicts.set(conflictKey(),result);else if(!syncError)remoteConflicts.delete(conflictKey());
   if(!state.dirty&&!recordDraft.dirty&&(rerender||listChanged)){render();if(block){const open=new CustomEvent('qingjian:restore-editor',{detail:{block,caret,end}});document.dispatchEvent(open);}}
   renderSaveState();
  }finally{network=null;unlock();}
  if(result?.status==='conflict'){controller.pause();if(!state.dirty&&!recordDraft.dirty&&!document.querySelector('.modal,.property-popover')){if(remoteConflicts.has(conflictKey()))showSyncConflict(remoteConflicts.get(conflictKey()));else showPending();}}
  else if(state.dirty||recordDraft.dirty){await flush();controller.request('edit');}
  if(syncError&&result?.status!=='conflict')throw syncError;return result;
 }
 async function showPending(){
  if(conflicts.length){await showDraftConflict();return;}
  if(remoteConflicts.has(conflictKey())){await manualSync();return;}
  modal(t('autoSyncTitle'),`<p>${e(label())}</p>${controller.entry().error?`<p class="report-warning">${e(controller.entry().error)}</p>`:''}<p class="form-hint">${t('autoSyncHint')}</p><div class="modal-footer"><button class="button" data-action="autoSyncRetry">${t('autoSyncRetry')}</button></div>`);
 }
 async function showDraftConflict(){
  if(state.busy||!conflicts.length)return;
  const item=conflicts[0],id=context().id,live=item.source==='page'?state.page:item.source==='record'?recordDraft.page:null;
  state.busy=true;item.draft=structuredClone(live||item.draft);render();
  try{item.preview=await callAPI('syncDraftPreview',{workspaceId:id,draft:item.draft,sourceCommit:item.sourceCommit});}finally{state.busy=false;render();}
  if(id!==context().id||conflicts[0]!==item)return;
  const preview=item.preview;modal(t('autoSyncDraftTitle'),`<form id="auto-sync-draft-form"><p class="form-hint">${t('autoSyncDraftHint')}</p><div class="sync-version-grid"><div><h4>${t('syncLocal')}</h4><pre>${e(draftSummary(preview.draft))}</pre></div><div><h4>${t('autoSyncDownloaded')}</h4><pre>${e(draftSummary(preview.remote))}</pre></div></div><label class="form-field">${t('syncChooseVersion')}<select name="choice" required><option value="">${t('syncNotChosen')}</option><option value="local">${t('syncKeepLocal')}</option><option value="remote">${t('syncKeepRemote')}</option><option value="copy">${t('autoSyncKeepBoth')}</option></select></label><p class="sync-operation-error" hidden></p><div class="modal-footer"><button type="button" class="button" data-action="closeModal">${t('cancel')}</button><button type="button" class="button" data-action="autoSyncStatus">${t('syncRecheck')}</button><button type="submit" class="button primary">${t('confirm')}</button></div></form>`,true);
  const form=document.querySelector('#auto-sync-draft-form');form.onsubmit=async event=>{event.preventDefault();if(state.busy)return;state.busy=true;form.querySelector('[type=submit]').disabled=true;
   try{const choice=form.elements.choice.value,result=await callAPI('resolveSyncDraft',{workspaceId:id,backupId:preview.id,choice});if(id!==context().id)return;
    if(item.source==='page'){const previous=state.page?.id;state.dirty=false;state.page=choice==='copy'?preview.remote:result.page;if(!state.page||state.page.deleted){state.page=null;state.route='home';state.recordPeek=null;}else{state.page=structuredClone(state.page);state.route=state.page.id;}localStorage.removeItem(`qingjian:recovery:${id}:${previous}`);}
    else if(item.source==='record'){recordDraft.dirty=false;recordDraft.onChange();recordDraft.clear();}
    conflicts.shift();state.saveError='';const pages=await callAPI('pages',{workspaceId:id});if(id!==context().id)return;state.pages=pages;if(!conflicts.length){remoteConflicts.delete(conflictKey());controller.resume();}closeModal();render();if(conflicts.length)await showDraftConflict();
   }catch(error){const target=form.querySelector('.sync-operation-error');target.hidden=false;target.textContent=errorMessage(error);}
   finally{state.busy=false;form.querySelector('[type=submit]').disabled=false;}
  };
 }
 function draftSummary(page){if(!page)return t('syncVersionDeleted');return (page.title||t('emptyTitle'))+'\n'+(page.blocks||[]).map(b=>b.text).join('\n').slice(0,3000)+(page.database?'\n'+t('database')+'\n'+JSON.stringify(page.database.rows).slice(0,3000):'');}
 function settingsHTML(w){return `<section class="auto-sync-settings"><label><input id="auto-sync-enabled" type="checkbox" ${w.autoSync!==false?'checked':''}> ${t('autoSyncTitle')}</label><p class="form-hint">${t('autoSyncHint')}</p><button type="button" class="button plain" data-action="autoSyncBackups">${t('autoSyncDraftBackups')}</button></section>`;}
 function bindSettings(){const input=document.querySelector('#auto-sync-enabled');if(!input)return;const id=context().id;input.onchange=async()=>{const enabled=input.checked;input.disabled=true;try{applyConfig(await callAPI('autoSyncSettings',{workspaceId:id,enabled}));if(id===context().id){controller.resume();redraw();}}catch(error){input.checked=!enabled;toast(errorMessage(error),true);}finally{input.disabled=false;}};}
 async function showBackups(){
  if(state.busy)return;const id=context().id,previous=document.querySelector('.modal');state.busy=true;render();let list;
  try{list=await callAPI('syncDraftBackups',{workspaceId:id});if(id!==context().id||(previous&&!previous.isConnected))return;}finally{state.busy=false;render();}
  modal(t('autoSyncDraftBackups'),`<p class="form-hint">${t('autoSyncDraftBackupHint')}</p><div class="managed-workspaces">${list.map(item=>`<section><strong>${e(item.title)}</strong><small>${e(new Date(item.createdAt).toLocaleString(t('dateFormat')))}</small><button class="button" data-draft-restore="${e(item.id)}">${t('autoSyncRestoreCopy')}</button></section>`).join('')||`<p>${t('historyEmpty')}</p>`}</div>`);
  document.querySelectorAll('[data-draft-restore]').forEach(button=>button.onclick=async()=>{
   if(state.busy||id!==context().id)return;state.busy=true;button.disabled=true;render();
   try{await callAPI('restoreSyncDraft',{workspaceId:id,backupId:button.dataset.draftRestore});controller.request('edit',id);if(id!==context().id||!button.isConnected)return;const pages=await callAPI('pages',{workspaceId:id});if(id!==context().id||!button.isConnected)return;state.pages=pages;toast(t('autoSyncRestored'));closeModal();}
   catch(error){if(id===context().id&&button.isConnected)toast(errorMessage(error),true);}
   finally{state.busy=false;button.disabled=false;render();}
  });
 }
 document.addEventListener('compositionstart',()=>composing=true);document.addEventListener('compositionend',()=>composing=false);
 document.addEventListener('visibilitychange',()=>{if(!document.hidden){controller.request('foreground');controller.tick();}});window.addEventListener('focus',()=>{controller.request('foreground');controller.tick();});window.addEventListener('online',()=>{controller.request('online');controller.tick();});
 return {controller,statusHTML,settingsHTML,bindSettings,showBackups,showPending,redraw,get network(){return network;},get draftBlocked(){return conflicts.length>0;},edit:()=>controller.request('edit'),start:()=>controller.start(),retry:()=>{closeModal();controller.resume();controller.tick();},acceptManual:result=>{if(result.status==='conflict'){remoteConflicts.set(conflictKey(),result);controller.pause();}else{remoteConflicts.delete(conflictKey());const entry=controller.entry();entry.paused=false;entry.failures=0;entry.retryAt=0;entry.pending=false;entry.status='current';entry.due=Date.now()+120000;controller.publish(entry);}}};
}
