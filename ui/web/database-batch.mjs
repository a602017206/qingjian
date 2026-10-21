import {clone,uid} from './text.mjs';
import {t} from './i18n.mjs';
import {normalizePropertyValue} from './property-edit.mjs';
import {buttonWritableTypes,systemPropertyTypes,readPersons} from './property-types.mjs';
import {readSelection,isCalendarDate,calendarCells,databaseCSV} from './database.mjs';
import {resolveViewOptions,queryDatabase} from './database-views.mjs';
export const collectionPropertyTypes=['multiselect','person','relation'];
export const batchPropertyTypes=[...buttonWritableTypes,'relation'];
const fail=key=>{throw new Error(t(key));};
export class RecordSelection{
 constructor(){this.key='';this.visible=[];this.selected=new Set();this.anchor='';}
 sync(key,visible){if(this.key!==key){this.clear();this.key=key;}this.visible=[...new Set(visible)];this.selected=new Set(this.visible.filter(id=>this.selected.has(id)));if(!this.visible.includes(this.anchor))this.anchor='';return this.ids();}
 ids(){return this.visible.filter(id=>this.selected.has(id));}
 clear(){this.selected.clear();this.anchor='';}
 replace(ids){this.selected=new Set(this.visible.filter(id=>ids.includes(id)));this.anchor=this.ids()[0]||'';}
 all(checked=true){this.selected=new Set(checked?this.visible:[]);this.anchor=this.visible[0]||'';}
 choose(id,{range=false,checked=true}={}){if(!this.visible.includes(id))return;if(range&&this.anchor){const a=this.visible.indexOf(this.anchor),b=this.visible.indexOf(id);for(const next of this.visible.slice(Math.min(a,b),Math.max(a,b)+1))checked?this.selected.add(next):this.selected.delete(next);}else{checked?this.selected.add(id):this.selected.delete(id);this.anchor=id;}}
}
export function batchEditableColumns(database){return database.columns.filter((c,i)=>i>0&&batchPropertyTypes.includes(c.type));}
export function selectedDatabaseRows(page,ids){if(!page?.database||!Array.isArray(ids)||!ids.length||ids.length>10000||new Set(ids).size!==ids.length)fail('batchRecordSelectionInvalid');const wanted=new Set(ids),rows=page.database.rows.filter(row=>wanted.has(row.id));if(rows.length!==wanted.size)fail('batchRecordSelectionInvalid');return rows;}
export function batchPropertyOptions(page,column,pages=[]){
 if(column.type==='relation'){const target=pages.find(p=>p.id===column.targetDatabaseId&&!p.deleted&&p.database);return (target?.database.rows||[]).filter(row=>!pages.some(p=>p.id===row.pageId&&p.deleted)).map(row=>({value:row.id,label:row.values[target.database.columns[0].id]||t('emptyRecord')}));}
 const values=new Set(column.options||[]);for(const row of page.database.rows){const raw=row.values[column.id];if(!raw)continue;if(collectionPropertyTypes.includes(column.type)){for(const value of column.type==='person'?readPersons(raw):readSelection(raw))values.add(value);}else values.add(raw);}return [...values].map(value=>({value,label:value}));
}
function patchValues(page,rows,column,operation,pages){
 const multiple=collectionPropertyTypes.includes(column.type),mode=operation.mode||'replace';
 if(!['replace','clear',...(multiple?['add','remove']:[])].includes(mode))fail('batchRecordModeInvalid');
 if(mode==='clear'){for(const row of rows)row.values[column.id]=multiple?'[]':'';return;}
 const raw=String(operation.value??'');if(!raw.trim())fail('batchRecordValueRequired');
 let value=multiple?JSON.stringify(column.type==='person'?readPersons(raw):readSelection(raw)):normalizePropertyValue(column.type,raw);
 if(column.type==='date'&&!isCalendarDate(value))fail('viewDateRequired');
 if(['select','status',...collectionPropertyTypes].includes(column.type)){
  const allowed=new Set(batchPropertyOptions(page,column,pages).map(item=>item.value)),values=multiple?JSON.parse(value):[value];if(!values.length||values.some(v=>!allowed.has(v)))fail('batchRecordOptionInvalid');
 }
 for(const row of rows){if(multiple&&mode!=='replace'){const before=column.type==='person'?readPersons(row.values[column.id]):readSelection(row.values[column.id]),items=JSON.parse(value);row.values[column.id]=JSON.stringify(mode==='add'?[...new Set([...before,...items])]:before.filter(v=>!items.includes(v)));}else row.values[column.id]=value;}
}
export function batchDatabase(page,ids,operation,pages=[]){
 const selected=selectedDatabaseRows(page,ids);if(page.locked||page.deleted||selected.some(row=>row.pageId&&pages.some(p=>p.id===row.pageId&&(p.locked||p.deleted))))fail('cannotModify');
 const copy=clone(page),db=copy.database,wanted=new Set(ids),rows=db.rows.filter(row=>wanted.has(row.id));
 if(operation.kind==='set'){const column=batchEditableColumns(db).find(c=>c.id===operation.propertyId);if(!column)fail('batchRecordReadonly');patchValues(copy,rows,column,operation,pages);return {page:copy,ids:[...ids]};}
 if(operation.kind==='duplicate'){
  if(rows.some(row=>row.pageId))fail('batchRecordCopyLinked');const created=[];
  db.rows=db.rows.flatMap(row=>{if(!wanted.has(row.id))return [row];const next=clone(row);next.id=uid();for(const key of ['sequence','createdAt','createdBy','updatedAt','updatedBy'])delete next[key];for(const c of db.columns)if([...systemPropertyTypes,'formula','rollup','button'].includes(c.type))delete next.values[c.id];next.values[db.columns[0].id]=(row.values[db.columns[0].id]||t('emptyRecord'))+' · '+t('copySuffix');created.push(next.id);return [row,next];});return {page:copy,ids:created};
 }
 if(operation.kind==='remove'){db.rows=db.rows.filter(row=>!wanted.has(row.id));return {page:copy,ids:[]};}
 fail('batchRecordModeInvalid');
}
export function visibleDatabaseRows(page,pages,options={}){options=resolveViewOptions(page,options);const rows=queryDatabase(page,pages,options).rows;if(options.view!=='calendar')return rows;const dateColumn=page.database.columns.find(c=>c.id===options.dateColumnId)||page.database.columns.find(c=>c.type==='date');if(!dateColumn)return [];const days=new Set(calendarCells(options.month));return rows.filter(row=>!isCalendarDate(row.values[dateColumn.id])||days.has(row.values[dateColumn.id]));}
export function selectedDatabaseCSV(page,ids,pages){const rows=selectedDatabaseRows(page,ids);return databaseCSV({...page,database:{...page.database,rows}},pages);}
