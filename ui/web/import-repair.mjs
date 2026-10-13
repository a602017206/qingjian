import { clone,descendants } from './core.mjs';
import { isCalendarDate } from './database.mjs';
export const optionPalette=['gray','green','purple','yellow','red','blue','brown','pink','orange'];
export function optionColor(column,value){const saved=column.optionColors?.[value];if(optionPalette.includes(saved))return saved;if(column.type==='status'){if(['未观看','未开始','待开始','待办','未完成'].includes(value))return 'red';if(['观看中','进行中','处理中','阅读中'].includes(value))return 'blue';if(['已观看','已完成','完成','已读'].includes(value))return 'green';if(['弃剧','取消','已取消','暂停'].includes(value))return 'gray';}const hash=Array.from(value).reduce((n,c)=>(Math.imul(n,31)+c.codePointAt(0))>>>0,0);return optionPalette[1+hash%(optionPalette.length-1)];}
export function inferredType(name,values){
 if(name.includes('状态')||['是否观看','观看状态','阅读进度','任务进度','完成情况'].includes(name))return 'status';
 if(['类型','分类','类别','地区','平台','标签'].includes(name))return values.some(v=>/[,，]/.test(v))?'multiselect':'select';
 if((name.includes('日期')||['观看时间','阅读时间','完成时间','开始时间','截止时间'].includes(name))&&values.every(v=>isCalendarDate(normalizeDate(v))))return 'date';
 if(['优先级','评分','数量','单价','金额','工时'].includes(name)&&values.every(v=>Number.isFinite(Number(v))))return 'number';return 'text';
}
function normalizeDate(value){const m=value.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})$/);return m?`${m[1]}-${m[2].padStart(2,'0')}-${m[3].padStart(2,'0')}`:value;}
export function importRepairSuggestions(pages,preferredId=''){
 const active=pages.filter(p=>!p.deleted),fields=[],parents=[];
 for(const original of active.filter(p=>p.database&&!p.locked&&(!preferredId||p.id===preferredId))){
  const page=clone(original),changes=[];
  page.database.columns.forEach((column,i)=>{if(i===0||column.type!=='text')return;const values=page.database.rows.map(r=>r.values[column.id]??'').filter(v=>v.trim());const type=inferredType(column.name,values);if(type==='text')return;changes.push({name:column.name,type});column.type=type;
   if(['status','select','multiselect'].includes(type)){column.options=[...new Set(values.flatMap(v=>type==='multiselect'?v.split(/[,，]/).map(v=>v.trim()).filter(Boolean):[v]))];column.optionColors=Object.fromEntries(column.options.map(v=>[v,optionColor(column,v)]));}
   for(const row of page.database.rows){const raw=row.values[column.id]??'';row.sourceValues??={};if(!Object.hasOwn(row.sourceValues,column.id))row.sourceValues[column.id]=raw;if(type==='multiselect')row.values[column.id]=JSON.stringify(raw.split(/[,，]/).map(v=>v.trim()).filter(Boolean));if(type==='date'&&raw)row.values[column.id]=normalizeDate(raw);}
  });
  if(changes.length)fields.push({pageId:page.id,page,changes});
 }
 const roots=active.filter(p=>!p.parentId&&!p.database&&!p.locked);
 for(const child of roots){if(!child.title.trim()||roots.filter(p=>p.title===child.title).length!==1)continue;const candidates=active.filter(p=>p.database&&(!preferredId||p.id===preferredId)&&p.id!==child.id&&p.database.rows.filter(r=>r.values[p.database.columns[0]?.id]===child.title).length===1&&!descendants(active,child.id).has(p.id));if(candidates.length===1)parents.push({pageId:child.id,parentId:candidates[0].id,title:child.title,parentTitle:candidates[0].title});}
 return {fields,parents};
}
