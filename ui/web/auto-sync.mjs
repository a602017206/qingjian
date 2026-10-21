import {mergeSavedMetadata} from './property-edit.mjs';
const copy=value=>value==null?value:structuredClone(value);
const metadata=['revision','createdAt','createdBy','updatedAt','updatedBy'];
export function syncPageContent(page){if(!page)return null;const result=copy(page);for(const key of metadata)delete result[key];if(result.database){delete result.database.nextSequence;for(const row of result.database.rows||[])for(const key of [...metadata,'sequence'])delete row[key];}const stable=value=>Array.isArray(value)?value.map(stable):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,stable(value[key])])):value;return JSON.stringify(stable(result));}
export function reconcileSyncDraft(base,draft,remote){
 const before=syncPageContent(base),local=syncPageContent(draft),after=syncPageContent(remote);
 if(local===before||local===after)return {kind:'clean',page:copy(remote)};
 if(remote&&after===before){const page=copy(draft);mergeSavedMetadata(page,remote);return {kind:'local',page};}
 return {kind:'conflict',page:copy(draft)};
}
export class AutoSyncController{
 constructor({context,run,onChange=()=>{},now=Date.now}){Object.assign(this,{context,run,onChange,now});this.entries=new Map();this.running=false;this.state={status:'idle'};}
 entry(c=this.context()){const key=c.id+'|'+c.remote;if(!this.entries.has(key))this.entries.set(key,{id:c.id,status:'pending',pending:true,due:this.now()+1200,failures:0,paused:false,retryAt:0});return this.entries.get(key);}
 publish(entry){this.state={...entry};this.onChange(this.state);}
 request(reason='edit',id=this.context().id){const c=this.context();if(c.id!==id)return;const e=this.entry(c);e.pending=true;if(e.paused)return;e.due=Math.max(e.retryAt||0,this.now()+(reason==='edit'?4000:reason==='open'?1200:0));if(e.status!=='syncing'&&e.status!=='error')e.status='pending';this.publish(e);}
 resume(){const e=this.entry();e.paused=false;e.failures=0;e.retryAt=0;this.request('manual');}
 pause(error=''){const e=this.entry();e.paused=true;e.status='conflict';e.error=error;this.publish(e);}
 async tick(){const c=this.context();if(!c.id)return;const e=this.entry(c);if(c.enabled===false||!c.remote){if(e.status!=='disabled'){e.status='disabled';this.publish(e);}return;}if(this.running||e.paused||!c.visible)return;if(!c.online){if(e.status!=='offline'){e.status='offline';this.publish(e);}return;}if(c.busy||this.now()<e.due)return;
  this.running=true;e.pending=false;e.status='syncing';this.publish(e);
  try{const result=await this.run(c.id);if(result?.status==='conflict'){e.paused=true;e.status='conflict';}else if(result?.status==='deferred'){e.pending=true;e.status='pending';e.due=this.now()+1000;}else{e.failures=0;e.retryAt=0;e.error='';e.lastSuccess=this.now();e.status=e.pending?'pending':'current';e.due=this.now()+(e.pending?4000:120000);}}
  catch(error){e.error=error.message;e.failures++;e.pending=true;e.retryAt=this.now()+Math.min(15000*2**(e.failures-1),300000);e.due=e.retryAt;e.status='error';}
  finally{this.running=false;if(this.context().id===c.id&&this.context().remote===c.remote)this.publish(e);}
 }
 start(){if(this.timer)return;this.tick();this.timer=setInterval(()=>this.tick(),1000);}
 stop(){clearInterval(this.timer);this.timer=null;}
}
