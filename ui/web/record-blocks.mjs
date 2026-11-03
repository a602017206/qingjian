import {uid,clone} from './text.mjs';
import {toMarkdown} from './markdown.mjs';
import {t} from './i18n.mjs';
export function cloneRecordBlocks(blocks){const ids=new Map(blocks.map(block=>[block.id,uid()]));return clone(blocks).map(block=>({...block,id:ids.get(block.id),...(block.parentId?{parentId:ids.get(block.parentId)||''}:{})}));}
export function hasRecordBodyConflict(row){return Array.isArray(row.blocks)&&typeof row.blockBody==='string'&&String(row.body??'')!==row.blockBody;}
export function resolveRecordBodyConflict(row){if(!hasRecordBodyConflict(row))return;row.blocks=[...row.blocks,{id:uid(),type:'paragraph',text:String(row.body??'')}];row.body=toMarkdown(row.blocks);row.blockBody=row.body;}
export class RecordBlockSession{
 constructor(){this.sessions=new WeakMap();}
 page(owner,row){
  let entry=this.sessions.get(row);if(entry?.owner===owner)return entry.page;
  const legacy=[{id:uid(),type:'paragraph',text:String(row.body??'')}];entry={owner,pending:null,baseline:JSON.stringify(row.blocks||legacy)};
  entry.page={id:owner.id+'-record-'+row.id,get title(){return row.values[owner.database.columns[0].id]||t('emptyRecord');},get locked(){return !!owner.locked||!!owner.deleted||hasRecordBodyConflict(row);},get blocks(){return entry.pending||row.blocks||legacy;},set blocks(value){entry.pending=value;}};
  this.sessions.set(row,entry);return entry.page;
 }
 commit(owner,row){
  const page=this.page(owner,row),entry=this.sessions.get(row),serialized=JSON.stringify(page.blocks);if(!entry.pending&&serialized===entry.baseline)return false;
  if(page.locked)throw new Error(t('cannotModify'));row.blocks=page.blocks;row.body=toMarkdown(row.blocks);row.blockBody=row.body;entry.pending=null;entry.baseline=serialized;return true;
 }
}
