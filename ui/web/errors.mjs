import {t} from './i18n.mjs';
export function errorMessage(failure,secrets=[]){
 let value=failure;for(let depth=0;depth<4&&value&&typeof value==='object';depth++)value=value.message??value.error??value.cause;
 let message=typeof value==='string'?value.trim():'';
 if(!message||message==='undefined'||message==='null')message=t('operationFailed');
 for(const secret of secrets)if(typeof secret==='string'&&secret)message=message.split(secret).join(t('hiddenCredential'));
 return message.replace(/(https?:\/\/)[^\s/@]+@/gi,`$1${t('hiddenCredential')}@`).replace(/([?&#](?:access_token|private_token|token|password|authorization|auth)=)[^&#\s]+/gi,`$1${t('hiddenCredential')}`).replace(/\b(Bearer|Basic)\s+[A-Za-z0-9._~+\/=:-]+/gi,`$1 ${t('hiddenCredential')}`);
}
export function operationError(failure,request={}){return new Error(errorMessage(failure,[request.token,request.password]));}
