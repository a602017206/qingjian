import { uid,clone } from './text.mjs';
import { computeCell,formatValue,isCalendarDate } from './database.mjs';
import { t } from './i18n.mjs';
export const viewTypes=['table','board','list','gallery','calendar'];
const emptyOps=['empty','not_empty'],numericOps=['gt','gte','lt','lte'],dateOps=['before','after','on_or_before','on_or_after'];
export function filterOperators(column){
 const type=column?.type;
 if(type==='checkbox')return ['eq','ne'];
 if(['multiselect','person','relation','files'].includes(type))return ['contains','not_contains',...emptyOps];
 if(['number','uniqueId'].includes(type))return ['eq','ne',...numericOps,...emptyOps];
 if(['date','createdTime','editedTime'].includes(type))return ['eq','ne',...dateOps,...emptyOps];
 if(['select','status'].includes(type))return ['eq','ne',...emptyOps];
 if(['formula','rollup'].includes(type))return ['eq','ne','contains','not_contains',...numericOps,...emptyOps];
 return ['eq','ne','contains','not_contains','starts_with','ends_with',...emptyOps];
}
export const emptyFilter=()=>({conjunction:'and',rules:[]});
function checkRule(rule,db){const column=db.columns.find(c=>c.id===rule.propertyId);if(!column||!filterOperators(column).includes(rule.operator)||typeof rule.value!=='string'||rule.value.length>4096)throw new Error(t('invalidViewRule'));
 if(!emptyOps.includes(rule.operator)){
  if((['number','uniqueId'].includes(column.type)||numericOps.includes(rule.operator))&&(!rule.value.trim()||!Number.isFinite(Number(rule.value))))throw new Error(t('viewNumberRequired'));
  if((['date','createdTime','editedTime'].includes(column.type)||dateOps.includes(rule.operator))&&!isCalendarDate(rule.value))throw new Error(t('viewDateRequired'));
  if(column.type==='checkbox'&&!['true','false'].includes(rule.value))throw new Error(t('invalidViewRule'));
 }
}
export function validateDatabaseViews(db){
 if(db.views===undefined)return;
 if(!Array.isArray(db.views)||!db.views.length||db.views.length>32)throw new Error(t('viewLimit'));
 const ids=new Set(),names=new Set();
 for(const view of db.views){
  if(!view||!/^[-\w]+$/.test(view.id)||ids.has(view.id)||typeof view.name!=='string'||!view.name.trim()||view.name.length>120||names.has(view.name.trim())||!viewTypes.includes(view.type))throw new Error(t('invalidDatabaseView'));ids.add(view.id);names.add(view.name.trim());
  let count=0;const rules=new Set();const visit=(node,depth=0)=>{if(depth>3||++count>64||!node||!Array.isArray(node.rules)||!['and','or'].includes(node.conjunction))throw new Error(t('viewFilterLimit'));for(const child of node.rules){if(!child||!/^[-\w]+$/.test(child.id)||rules.has(child.id))throw new Error(t('invalidViewRule'));rules.add(child.id);if('rules'in child)visit(child,depth+1);else{if(++count>64)throw new Error(t('viewFilterLimit'));checkRule(child,db);}}};visit(view.filter);
  if(!Array.isArray(view.sorts)||view.sorts.length>8||new Set(view.sorts.map(s=>s.propertyId)).size!==view.sorts.length||view.sorts.some(s=>!db.columns.some(c=>c.id===s.propertyId)||!['asc','desc'].includes(s.direction)))throw new Error(t('invalidViewSort'));
  if(!Array.isArray(view.hiddenColumnIds)||new Set(view.hiddenColumnIds).size!==view.hiddenColumnIds.length||view.hiddenColumnIds.some(id=>id===db.columns[0]?.id||!db.columns.some(c=>c.id===id)))throw new Error(t('invalidViewColumns'));
  if(view.groupColumnId&&!db.columns.some(c=>c.id===view.groupColumnId&&['select','status'].includes(c.type))||view.dateColumnId&&!db.columns.some(c=>c.id===view.dateColumnId&&c.type==='date'))throw new Error(t('invalidViewRule'));
 }
}
export function makeDatabaseView(name,type='table'){return {id:uid(),name,type,filter:emptyFilter(),sorts:[],hiddenColumnIds:[],groupColumnId:'',dateColumnId:''};}
export function createDatabaseView(db,{name,type='table',baseType='table'}={}){const original=db.views?.length?db.views:[makeDatabaseView(t('allRecords'),baseType)],view=makeDatabaseView(name,type),candidate={...db,views:[...original,view]};validateDatabaseViews(candidate);db.views=candidate.views;return view;}
export function duplicateDatabaseView(db,id){const original=db.views?.find(v=>v.id===id);if(!original)throw new Error(t('invalidDatabaseView'));const view=clone(original);view.id=uid();view.name=original.name+' · '+t('copySuffix');for(let i=2;db.views.some(v=>v.name===view.name);i++)view.name=original.name+' · '+t('copySuffix')+' '+i;const renew=node=>{if(node.id)node.id=uid();node.rules?.forEach(renew);};renew(view.filter);validateDatabaseViews({...db,views:[...db.views,view]});db.views.push(view);return view;}
export function resolveViewOptions(page,options={}){const view=page.database.views?.find(v=>v.id===options.activeViewId)||page.database.views?.[0];return view?{...options,activeViewId:view.id,view:view.type,filter:view.filter,sorts:view.sorts,hiddenColumnIds:view.hiddenColumnIds,groupColumnId:view.groupColumnId,dateColumnId:view.dateColumnId}:{...options,view:options.view||'table'};}
export function pruneViewReferences(db){let removed=0;for(const view of db.views||[]){const prune=node=>{node.rules=node.rules.filter(rule=>{if(rule.rules){prune(rule);if(!rule.rules.length){removed++;return false;}return true;}try{checkRule(rule,db);return true;}catch{removed++;return false;}});};prune(view.filter);const filter=(list,predicate)=>list.filter(value=>predicate(value)||(removed++,false));view.sorts=filter(view.sorts,s=>db.columns.some(c=>c.id===s.propertyId));view.hiddenColumnIds=filter(view.hiddenColumnIds,id=>db.columns.some(c=>c.id===id)&&id!==db.columns[0]?.id);for(const [key,types] of [['groupColumnId',['select','status']],['dateColumnId',['date']]])if(view[key]&&!db.columns.some(c=>c.id===view[key]&&types.includes(c.type))){view[key]='';removed++;}}return removed;}
function normalizedDate(value,type){const source=String(value),match=source.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})(.*)$/);if(!match)throw new Error(t('viewDateRequired'));const day=match[1]+'-'+match[2].padStart(2,'0')+'-'+match[3].padStart(2,'0');if(!isCalendarDate(day))throw new Error(t('viewDateRequired'));if(match[4]&&!Number.isFinite(Date.parse(source)))throw new Error(t('viewDateRequired'));return type==='date'?day:match[4]?new Date(source).toISOString():day;}
export function queryDatabase(page,pages,options={}){
 const db=page.database,errors=new Set(),cache=new Map();
 const get=(row,id)=>{const key=row.id+'\0'+id;if(!cache.has(key)){const column=db.columns.find(c=>c.id===id);let result;if(!column)result={value:null,error:t('invalidViewRule')};else if(column.type==='number'&&!String(row.values[id]??'').trim())result={value:null,error:''};else result=computeCell(page,row,id,pages);
  if(column&&!result.error){try{if(column.type==='location'&&result.value&&!result.value.address&&result.value.latitude===undefined&&result.value.longitude===undefined)result.value=null;
   if(column.type==='uniqueId'){const raw=String(result.value??''),prefix=column.uniqueIdPrefix?column.uniqueIdPrefix+'-':'',source=raw.startsWith(prefix)?raw.slice(prefix.length):raw;const number=Number.isSafeInteger(row.sequence)&&row.sequence>0?row.sequence:Number(source);if(!raw)result.value=null;else if(!Number.isSafeInteger(number)||number<=0)throw new Error(t('viewNumberRequired'));else result.value=number;}
   if(['date','createdTime','editedTime'].includes(column.type)&&result.value)result.value=normalizedDate(result.value,column.type);
  }catch(error){result={value:null,error:column.name+'：'+error.message};}}
  cache.set(key,result);}const result=cache.get(key);if(result.error)errors.add(result.error);return result;};
 const empty=value=>value===null||value===undefined||value===''||Array.isArray(value)&&!value.length;
 const testRule=(row,rule)=>{let column;try{checkRule(rule,db);column=db.columns.find(c=>c.id===rule.propertyId);}catch(error){errors.add(error.message);return false;}const result=get(row,rule.propertyId);if(result.error)return false;const value=result.value,op=rule.operator,target=rule.value;if(op==='empty')return empty(value);if(op==='not_empty')return !empty(value);if(empty(value))return false;
  let a=value,b=target;if(column.type==='checkbox')b=target==='true';else if(typeof value==='number'||numericOps.includes(op)){a=Number(value);b=Number(target);}else if(['date','createdTime','editedTime'].includes(column.type)){a=String(value).slice(0,10);}else if(!Array.isArray(value)){a=formatValue(value).toLocaleLowerCase();b=target.toLocaleLowerCase();}
  switch(op){case 'eq':return a===b;case 'ne':return a!==b;case 'contains':case 'not_contains':{const found=Array.isArray(a)?a.some(item=>typeof item==='object'?formatValue(item).toLocaleLowerCase().includes(target.toLocaleLowerCase()):item===target):String(a).includes(String(b));return op==='contains'?found:!found;}case 'starts_with':return String(a).startsWith(String(b));case 'ends_with':return String(a).endsWith(String(b));case 'gt':case 'after':return a>b;case 'gte':case 'on_or_after':return a>=b;case 'lt':case 'before':return a<b;case 'lte':case 'on_or_before':return a<=b;default:return false;}
 };
 let filterValid=true;const verify=(node,depth=0)=>{if(depth>3||!node||!Array.isArray(node.rules)||!['and','or'].includes(node.conjunction))throw new Error(t('invalidViewRule'));for(const child of node.rules)child.rules?verify(child,depth+1):checkRule(child,db);};try{if(options.filter)verify(options.filter);}catch(error){filterValid=false;errors.add(error.message);}
 const configured=node=>node.rules.some(child=>child.rules?configured(child):true);
 const matches=(row,node)=>{if(!node)return true;const children=node.rules.filter(child=>!child.rules||configured(child));return !children.length||(node.conjunction==='or'?children.some(child=>child.rules?matches(row,child):testRule(row,child)):children.every(child=>child.rules?matches(row,child):testRule(row,child)));};
 const query=String(options.query||'').toLocaleLowerCase();let rows=db.rows.filter(row=>filterValid&&matches(row,options.filter)&&(!query||[...Object.values(row.values),row.body||'',...db.columns.map(c=>formatValue(get(row,c.id).value))].join(' ').toLocaleLowerCase().includes(query)));
 const sorts=options.sorts|| (options.sort?[{propertyId:options.sort,direction:options.desc?'desc':'asc'}]:[]);for(const sort of sorts)if(!db.columns.some(c=>c.id===sort.propertyId))errors.add(t('invalidViewSort'));
 rows=rows.map((row,index)=>({row,index})).sort((left,right)=>{for(const sort of sorts){const column=db.columns.find(c=>c.id===sort.propertyId);if(!column)continue;const a=get(left.row,sort.propertyId).value,b=get(right.row,sort.propertyId).value;if(empty(a)||empty(b)){if(empty(a)!==empty(b))return empty(a)?1:-1;continue;}let order;if(typeof a==='number'&&typeof b==='number')order=a-b;else if(['select','status'].includes(column.type)){const ai=column.options?.indexOf(a)??-1,bi=column.options?.indexOf(b)??-1;order=ai>=0&&bi>=0?ai-bi:ai>=0?-1:bi>=0?1:formatValue(a).localeCompare(formatValue(b),'zh');}else order=formatValue(a).localeCompare(formatValue(b),'zh',{numeric:true});if(order)return sort.direction==='desc'?-order:order;}return left.index-right.index;}).map(item=>item.row);
 return {rows,errors:[...errors]};
}
