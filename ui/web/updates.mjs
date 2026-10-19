export const releaseRepository='a602017206/qingjian';
export const releasesURL=`https://github.com/${releaseRepository}/releases`;
const versionParts=value=>{const match=/^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:\+[0-9A-Za-z.-]+)?$/.exec(value);if(!match||value.length>128)throw new Error('updateInvalid');return match.slice(1,4).map(BigInt);};
export function compareVersions(a,b){const x=versionParts(a),y=versionParts(b);for(let i=0;i<3;i++)if(x[i]!==y[i])return x[i]>y[i]?1:-1;return 0;}
function platformFiles({os,arch,simulator}){
 if(arch==='aarch64'){if(os==='android')return ['qingjian-android-arm64.apk'];if(os==='macos')return ['qingjian-macos-arm64.zip'];if(os==='ios')return [simulator?'qingjian-ios-simulator-arm64.zip':'qingjian-ios-unsigned-arm64.ipa'];}
 if(arch==='x86_64'){if(os==='windows')return ['qingjian-windows-x64.exe'];if(os==='macos')return ['qingjian-macos-x64.zip'];if(os==='linux')return ['qingjian-linux-x64.AppImage','qingjian-linux-x64.deb'];}
 return [];
}
export function evaluateRelease(info,release){
 if(!release||release.draft||release.prerelease||!/^v\d+\.\d+\.\d+$/.test(release.tag_name)||!Array.isArray(release.assets))throw new Error('updateInvalid');
 const version=release.tag_name.slice(1),newer=compareVersions(version,info.version)>0;
 const pageURL=`${releasesURL}/tag/${release.tag_name}`;
 if(release.html_url!==pageURL)throw new Error('updateInvalid');
 const downloads=[];
 if(newer)for(const name of platformFiles(info)){
  const matches=release.assets.filter(asset=>asset.name===name);if(matches.length>1)throw new Error('updateInvalid');const asset=matches[0];if(!asset)continue;
  const url=`https://github.com/${releaseRepository}/releases/download/${release.tag_name}/${name}`;
  if(asset.browser_download_url!==url||!Number.isSafeInteger(asset.size)||asset.size<=0)throw new Error('updateInvalid');
  downloads.push({name,url,size:asset.size});
 }
 return {newer,version,pageURL,notes:typeof release.body==='string'?release.body.slice(0,30000):'',downloads};
}
export async function fetchLatestRelease(fetcher=globalThis.fetch){
 const controller=new AbortController(),timer=setTimeout(()=>controller.abort(),12000);
 try{
  const response=await fetcher(`https://api.github.com/repos/${releaseRepository}/releases/latest`,{headers:{Accept:'application/vnd.github+json'},credentials:'omit',referrerPolicy:'no-referrer',cache:'no-store',signal:controller.signal});
  if(!response.ok)throw new Error([403,429].includes(response.status)?'updateRateLimited':response.status===404?'updateNotPublished':'updateNetwork');
  if(Number(response.headers?.get('content-length'))>1048576)throw new Error('updateInvalid');
  const reader=response.body?.getReader();let text='';
  if(reader){const decoder=new TextDecoder();let size=0;try{for(;;){const {done,value}=await reader.read();if(done)break;size+=value.byteLength;if(size>1048576){await reader.cancel();throw new Error('updateInvalid');}text+=decoder.decode(value,{stream:true});}text+=decoder.decode();}finally{reader.releaseLock();}}
  else{text=await response.text();if(text.length>1048576)throw new Error('updateInvalid');}
  try{return JSON.parse(text);}catch{throw new Error('updateInvalid');}
 }catch(error){if(/^update[A-Z]/.test(error.message))throw error;throw new Error('updateNetwork');}finally{clearTimeout(timer);}
}
export class UpdateChecker{
 constructor({info,fetchRelease=fetchLatestRelease,onChange=()=>{}}){this.info=info;this.fetchRelease=fetchRelease;this.onChange=onChange;this.state={status:'idle'};this.pending=null;}
 check(){if(this.pending)return this.pending;this.state={status:'checking'};this.onChange(this.state);
 this.pending=Promise.resolve().then(async()=>{try{const info=await this.info(),release=await this.fetchRelease();this.state={...evaluateRelease(info,release),status:'current',info};if(this.state.newer)this.state.status='available';}catch(error){this.state={status:'error',error:/^update[A-Z]/.test(error.message)?error.message:'updateNetwork'};}this.onChange(this.state);return this.state;}).finally(()=>this.pending=null);return this.pending;
 }
}
