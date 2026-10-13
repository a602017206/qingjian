import { invoke, isTauri } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import { readFile, writeFile } from '@tauri-apps/plugin-fs';
import { t } from './i18n.mjs';
import { openUrl } from '@tauri-apps/plugin-opener';
import { pickedFilename } from './core.mjs';
export const native = () => isTauri();
export async function api(action,request={}){
 if(native())return await invoke('dispatch',{action,request});
 const response=await fetch(`/api/${action}`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(request)});
 if(!response.ok)throw new Error(t('operationFailed'));const result=await response.json();if(result.error)throw new Error(result.error);return result.data;
}
export function encodeBase64(bytes){let str='';for(let i=0;i<bytes.length;i+=16384)str+=String.fromCharCode(...bytes.subarray(i,i+16384));return btoa(str);}
export function decodeBase64(base64){return Uint8Array.from(atob(base64),c=>c.charCodeAt(0));}
export async function pickFile(attachment=false){
 if(native()){const path=await open({multiple:false,filters:attachment?[]:[{name:t('import'),extensions:['zip','md','markdown','csv','html','htm','json']}]});if(!path)return null;const bytes=await readFile(path);if(bytes.length>(attachment?32:128)*1024**2)throw new Error(t('fileTooLarge'));return {name:pickedFilename(path),data:encodeBase64(bytes)};}
 return new Promise((resolve,reject)=>{const input=document.createElement('input');input.type='file';if(!attachment)input.accept='.zip,.md,.markdown,.csv,.html,.htm,.json';input.oncancel=()=>resolve(null);input.onchange=async()=>{try{const f=input.files[0];if(!f){resolve(null);return;}if(f.size>(attachment?32:128)*1024**2)throw new Error(t('fileTooLarge'));resolve({name:f.name,data:encodeBase64(new Uint8Array(await f.arrayBuffer()))});}catch(e){reject(e)}};input.click();});
}
export async function saveFile(name,data){
 const bytes=decodeBase64(data);
 if(native()){
  const path=await save({defaultPath:name,filters:[{name:t('export'),extensions:[name.split('.').pop()]}]});if(!path)return null;await writeFile(path,bytes);return path;
 }
 const url=URL.createObjectURL(new Blob([bytes]));const a=document.createElement('a');a.href=url;a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(url),30000);return name;
}

export async function openExternal(url){if(native())await openUrl(url);else window.open(url,'_blank','noopener,noreferrer');}
