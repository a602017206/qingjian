import {clone,uid} from './text.mjs';
import {cloneRecordBlocks} from './record-blocks.mjs';
import {toMarkdown} from './markdown.mjs';
import {systemPropertyTypes} from './property-types.mjs';
import {t} from './i18n.mjs';
function retargetSelf(value,oldId,newId){if(!oldId)return value;if(typeof value==='string')return value.replace(/page:([A-Za-z0-9_-]+)/g,(match,id)=>id===oldId?'page:'+newId:match);if(Array.isArray(value))return value.map(item=>retargetSelf(item,oldId,newId));if(value&&typeof value==='object'){for(const key of Object.keys(value))value[key]=key==='targetDatabaseId'&&value[key]===oldId?newId:retargetSelf(value[key],oldId,newId);}return value;}
const readonly=new Set([...systemPropertyTypes,'formula','rollup','button']);
function freshPage(source){const page=clone(source);for(const key of ['createdAt','createdBy','updatedAt','updatedBy','deleted','isTemplate'])delete page[key];Object.assign(page,{id:uid(),revision:0,parentId:'',favorite:false,locked:false,blocks:cloneRecordBlocks(source.blocks||[])});if(page.database){page.database.rows=[];delete page.database.nextSequence;for(const template of page.database.templates||[])template.blocks=cloneRecordBlocks(template.blocks||[]);}return retargetSelf(page,source.id,page.id);}
export function pageTemplate(source,name){const page=freshPage(source);page.title=name.trim()||source.title;page.isTemplate=true;return page;}
export function instantiatePageTemplate(template){return freshPage(template);}
function writableValues(database,values){return Object.fromEntries(database.columns.filter(c=>!readonly.has(c.type)&&Object.hasOwn(values,c.id)).map(c=>[c.id,values[c.id]]));}
export function recordTemplate(database,row,blocks,name){return {id:uid(),name:name.trim()||t('emptyRecord'),values:clone(writableValues(database,row.values)),blocks:cloneRecordBlocks(blocks)};}
export function instantiateRecordTemplate(database,template,overrides={}){if(!template)throw new Error(t('templateMissing'));const blocks=cloneRecordBlocks(template.blocks||[]),body=toMarkdown(blocks);return {id:uid(),values:clone(writableValues(database,{...template.values,...overrides})),blocks,body,blockBody:body};}
