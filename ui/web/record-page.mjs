import { clone,escapeHTML as e } from './core.mjs';
import { renderCell } from './database-ui.mjs';
import { propertyTypeIcon,systemPropertyTypes } from './property-types.mjs';
import { icon } from './icons.mjs';
import { t } from './i18n.mjs';
import { computeCell,readSelection } from './database.mjs';

export function linkedRecord(page,pages){
 if(!page?.parentId)return null;
 const owner=pages.find(p=>p.id===page.parentId&&!p.deleted&&p.database);
 if(!owner)return null;
 const rows=owner.database.rows.filter(r=>r.pageId===page.id);
 return rows.length===1?{owner,row:rows[0]}:null;
}
export function recordValueDraft(page,pages,rowId,columnId,value){
 const linked=linkedRecord(page,pages);
 if(!linked||linked.row.id!==rowId||page.locked||linked.owner.locked||page.deleted)throw new Error(t('cannotModify'));
 const column=linked.owner.database.columns.find(c=>c.id===columnId);
 if(!column||column.id===linked.owner.database.columns[0]?.id||['formula','rollup','button',...systemPropertyTypes].includes(column.type))throw new Error(t('cannotModify'));
 const owner=clone(linked.owner);owner.database.rows.find(r=>r.id===rowId).values[columnId]=value;return owner;
}
export function recordSchemaDraft(page,pages,mutate){
 const linked=linkedRecord(page,pages);
 if(!linked||page.locked||linked.owner.locked||page.deleted)throw new Error(t('cannotModify'));
 const owner=clone(linked.owner);
 if(page.importSource&&!Array.isArray(owner.database.importColumns))owner.database.importColumns=clone(owner.database.columns);
 if(!Array.isArray(owner.database.propertyLayout?.order))owner.database.propertyLayout={...owner.database.propertyLayout,order:recordPropertyColumns(page,owner).map(c=>c.id)};
 mutate(owner.database);
 if(!linkedRecord(page,[owner]))throw new Error(t('cannotModify'));
 return owner;
}
export function recordPropertyColumns(page,owner){
 const columns=owner.database.columns.slice(1),configured=owner.database.propertyLayout?.order;
 const order=Array.isArray(configured)?configured:(page.importProperties||[]).map(p=>p.name);
 const position=c=>{const i=order.indexOf(Array.isArray(configured)?c.id:c.name);return i<0?order.length:i;};
 return columns.sort((a,b)=>position(a)-position(b));
}
function originalProperties(page,linked){
 const properties=new Map((page.importProperties||[]).map(p=>[p.name,String(p.raw??'')]));
 for(const column of (Array.isArray(linked.owner.database.importColumns)?linked.owner.database.importColumns:linked.owner.database.columns)){
  const raw=linked.row.sourceValues?.[column.id];
  if(!properties.has(column.name)&&raw!=null)properties.set(column.name,String(raw));
 }
 return properties;
}
export function visibleRecordBlocks(page,linked){
 const blocks=page.blocks||[];
 if(!linked||!page.importSource)return blocks;
 const properties=originalProperties(page,linked);
 const first=blocks[0];if(first?.type!=='paragraph')return blocks;
 const lines=first.text.split('\n').filter(line=>line.trim());
 if(!lines.length)return blocks;
 const names=new Set();
 for(const line of lines){
  const match=line.match(/^([^:：]+)[:：]\s*(.*)$/);
  if(!match||names.has(match[1].trim())||properties.get(match[1].trim())?.trim()!==match[2].trim())return blocks;
  names.add(match[1].trim());
 }
 const sourceColumns=Array.isArray(linked.owner.database.importColumns)?linked.owner.database.importColumns:linked.owner.database.columns;
 const required=(page.importProperties?.length? page.importProperties.map(p=>p.name):sourceColumns.slice(1).filter(c=>String(linked.row.sourceValues?.[c.id]??'').trim()).map(c=>c.name));
 if(!required.length||required.some(name=>!names.has(name)))return blocks;
 return blocks.slice(1);
}
const propertyGlyph=propertyTypeIcon;
export function renderRecordProperties(page,pages,disabled=false){
 const linked=linkedRecord(page,pages);if(!linked)return '';
 const {owner,row}=linked,locked=disabled||page.locked||owner.locked;
 const columns=recordPropertyColumns(page,owner).filter(column=>{
  const visibility=owner.database.propertyLayout?.visibility?.[column.id];if(visibility==='hidden')return false;
  if(visibility==='nonempty'){if(['formula','rollup','person','files','location',...systemPropertyTypes].includes(column.type)){const result=computeCell(owner,row,column.id,pages);if(column.type==='location'&&!result.error)return Boolean(result.value.address||result.value.latitude!==undefined);return result.error||!(result.value==null||result.value===''||(Array.isArray(result.value)&&!result.value.length));}const raw=row.values[column.id];if(['multiselect','relation'].includes(column.type)){try{return readSelection(raw).length>0;}catch{return true;}}return raw!=null&&raw!=='';}
  return true;
 });
 return `<section class="record-page-properties" aria-label="${t('recordProperties')}">${columns.map(column=>{
  let cell=renderCell(owner,row,column,pages,locked);
  cell=cell.replace('aria-label=',`placeholder="${t('emptyProperty')}" aria-label=`).replace(`<option value="">${t('none')}</option>`,`<option value="">${t('emptyProperty')}</option>`).replace(t('chooseValues'),t('emptyProperty'));
  return `<div class="record-property-row" data-property="${e(column.id)}"><button type="button" class="record-property-name" data-action="propertyMenu" data-column="${e(column.id)}" aria-haspopup="menu" ${locked?'disabled':''} title="${e(t(column.type==='number'?'numberProperty':column.type))}">${propertyGlyph(column.type)}<span>${e(column.name)}</span></button><div class="record-property-value">${cell}${!row.values[column.id]&&column.type==='date'?`<span class="property-empty">${t('emptyProperty')}</span>`:''}</div></div>`;
 }).join('')}<div class="record-property-tools"><button type="button" data-action="addColumn" ${locked?'disabled':''}>${icon('plus',15)}${t('addColumn')}</button><button type="button" data-action="propertyLayout" ${locked?'disabled':''}>${icon('grid',15)}${t('propertyLayout')}</button></div></section>`;
}

export class RecordDraftQueue {
 constructor({save,onSaved=()=>{},onChange=()=>{}}){this.save=save;this.onSaved=onSaved;this.onChange=onChange;this.page=null;this.dirty=false;this.saving=null;this.seq=0;this.error='';this.workspaceId='';this.childId='';}
 edit(workspaceId,child,pages,rowId,columnId,value){
  if(this.dirty&&(workspaceId!==this.workspaceId||child.id!==this.childId))throw new Error(t('unsavedWarning'));
  const context=pages.map(p=>this.page&&(this.dirty||this.saving)&&p.id===this.page.id?this.page:p);
  this.page=recordValueDraft(child,context,rowId,columnId,value);this.workspaceId=workspaceId;this.childId=child.id;this.dirty=true;this.seq++;this.error='';this.onChange();
 }
 editSchema(workspaceId,child,pages,mutate){
  if(this.dirty&&(workspaceId!==this.workspaceId||child.id!==this.childId))throw new Error(t('unsavedWarning'));
  const context=pages.map(p=>this.page&&(this.dirty||this.saving)&&p.id===this.page.id?this.page:p);
  this.page=recordSchemaDraft(child,context,mutate);this.workspaceId=workspaceId;this.childId=child.id;this.dirty=true;this.seq++;this.error='';this.onChange();
 }
 restore(workspaceId,childId,page){this.page=clone(page);this.workspaceId=workspaceId;this.childId=childId;this.dirty=true;this.seq++;this.error='';this.onChange();}
 clear(){if(this.dirty||this.saving)throw new Error(t('unsavedWarning'));this.onChange();this.page=null;this.workspaceId='';this.childId='';this.error='';}
 flush(){
  if(this.flushing)return this.flushing;
  if(!this.dirty||!this.page)return Promise.resolve();
  this.flushing=this.drain().finally(()=>{this.flushing=null;});return this.flushing;
 }
 async drain(){while(this.dirty){
  const page=clone(this.page),seq=this.seq;
  this.saving=this.save(this.workspaceId,page);this.onChange();
  try{
   const saved=await this.saving;this.page.revision=saved.revision;this.page.updatedAt=saved.updatedAt;this.page.createdAt=saved.createdAt;this.onSaved(saved);this.dirty=this.seq!==seq;this.error='';
  }catch(error){this.error=error.message;throw error;}finally{this.saving=null;this.onChange();}
 }}
}
