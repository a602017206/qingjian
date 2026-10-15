import { uid } from './text.mjs';
export { uid,escapeHTML,clone,safeURL } from './text.mjs';
export { inlineMarkdown,parseMarkdown,toMarkdown } from './markdown.mjs';
export function filterPages(pages,query=''){
 const terms=query.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
 return pages.filter(p=>!p.deleted&&terms.every(term=>[p.title,...p.tags??[],...(p.blocks??[]).map(b=>b.text),...(p.database?.rows??[]).flatMap(r=>Object.values(r.values))].join(' ').toLocaleLowerCase().includes(term)));
}
export function descendants(pages,id){const ids=new Set([id]);let changed=true;while(changed){changed=false;for(const p of pages)if(ids.has(p.parentId)&&!ids.has(p.id)){ids.add(p.id);changed=true;}}return ids;}
export function byteSize(n){if(n<1024)return `${n} B`;if(n<1024**2)return `${(n/1024).toFixed(1)} KB`;if(n<1024**3)return `${(n/1024**2).toFixed(1)} MB`;return `${(n/1024**3).toFixed(2)} GB`;}
export function makePage(title='未命名页面', parentId='') {return {title,icon:'📄',tags:[],blocks:[{id:uid(),type:'paragraph',text:''}],parentId,favorite:false,deleted:false,wide:false,locked:false};}
export function makeDatabase(){return {columns:[{id:'name',name:'名称',type:'text'},{id:'status',name:'状态',type:'select',options:['待开始','进行中','已完成']},{id:'date',name:'日期',type:'date'}],rows:[]};}
export function csvParse(text){const rows=[];let row=[],value='',quoted=false;for(let i=0;i<text.length;i++){const c=text[i];if(c==='"'){if(quoted&&text[i+1]==='"'){value+='"';i++;}else quoted=!quoted;}else if(c===','&&!quoted){row.push(value);value='';}else if(c==='\n'&&!quoted){row.push(value.replace(/\r$/,''));rows.push(row);row=[];value='';}else value+=c;}if(value||row.length){row.push(value.replace(/\r$/,''));rows.push(row);}if(quoted)throw new Error('表格中有未闭合的引号');return rows;}
export function pickedFilename(uri){
 let name=String(uri).replaceAll('\\','/').split('/').pop();
 if (/^(file|content):/i.test(uri)) { try{name=decodeURIComponent(name);}catch{} }
 return name||'导入笔记';
}
