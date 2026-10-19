import {t} from './i18n.mjs';
import {errorMessage} from './errors.mjs';
export function credentialFieldsHTML(){return `<label class="form-field credential-remember"><span><input name="remember" type="checkbox" disabled> ${t('credentialRemember')}</span><small>${t('credentialRememberHint')}</small></label><p class="form-hint credential-status" role="status">${t('credentialLoading')}</p><button type="button" class="button plain credential-forget" hidden>${t('credentialForget')}</button>`;}
export async function bindCredentialForm(form,api,workspaceId){
 const remember=form.elements.remember,status=form.querySelector('.credential-status'),forget=form.querySelector('.credential-forget'),initialRemote=form.elements.remote?.value;
 const active=()=>form.isConnected;
 try{
  const capabilities=await api('credentialCapabilities');if(!active())return;
  remember.dataset.available=String(capabilities.available===true);remember.disabled=!capabilities.available;remember.checked=capabilities.available===true;
  if(!workspaceId){status.textContent=t(capabilities.available?'credentialNotSaved':'credentialUnavailable');return;}
  const result=await api('credentialStatus',{workspaceId});if(!active()||form.elements.remote?.value!==initialRemote)return;
  if(!form.elements.username.value)form.elements.username.value=result.username||'';
  status.textContent=t(result.saved?'credentialSaved':result.session?'credentialSession':capabilities.available?'credentialNotSaved':'credentialUnavailable');forget.hidden=!result.session&&!result.saved;
 }catch(error){if(active())status.textContent=errorMessage(error);}
 forget.onclick=async()=>{forget.disabled=true;try{await api('forgetCredentials',{workspaceId});if(!active())return;form.elements.username.value='';form.elements.token.value='';forget.hidden=true;status.textContent=t('credentialForgotten');}catch(error){if(active())status.textContent=errorMessage(error);}finally{forget.disabled=false;}};
}
