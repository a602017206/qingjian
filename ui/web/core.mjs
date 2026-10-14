export const uid = () => globalThis.crypto?.randomUUID?.().replaceAll('-', '') ?? `${Date.now().toString(36)}${Math.random().toString(36).slice(2)}`;
export const escapeHTML = value => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
export const clone = value => JSON.parse(JSON.stringify(value));
export function safeURL(value) {
 const url=String(value??'').trim();
 if (/^(https?:\/\/|mailto:|page:[a-zA-Z0-9_-]+$|asset:[a-zA-Z0-9_.-]+$)/i.test(url)) return url;
 return '';
}
export function inlineMarkdown(value, pages=[]) {
 let text=escapeHTML(value);
 text=text.replace(/\[\[([^\]]+)\]\]/g, (_,name)=>{const p=pages.find(p=>!p.deleted&&p.title===name);return p?`<a href="page:${p.id}" class="page-link">↗ ${name}</a>`:`<span class="unresolved-link">${name}</span>`;});
 text=text.replace(/(!?)\[([^\]]*)\]\(([^)]+)\)/g, (_,image,label,target)=>{
  const url=safeURL(target);if(!url) return label;
  if(!image&&url.startsWith('asset:'))return `<span class="asset-link" data-asset="${escapeHTML(url.slice(6))}" data-label="${label}"></span>`;
  if(image) return url.startsWith('asset:')?`<span class="asset-preview" data-asset="${escapeHTML(url.slice(6))}"></span>`:`<span class="external-image">${label || '远程图片'} <a href="${escapeHTML(url)}">↗</a></span>`;
  return `<a href="${escapeHTML(url)}"${url.startsWith('http')?' target="_blank" rel="noopener noreferrer"':''}>${label}</a>`;
 });
 return text.replace(/`([^`]+)`/g,'<code>$1</code>').replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>').replace(/~~([^~]+)~~/g,'<s>$1</s>').replace(/\*([^*\n]+)\*/g,'<em>$1</em>').replaceAll('\n','<br>');
}
export function parseMarkdown(text) {
 const lines=text.replaceAll('\r\n','\n').split('\n'), blocks=[];let para=[];
 const flush=()=>{if(para.length){blocks.push({id:uid(),type:'paragraph',text:para.join('\n')});para=[];}};
 for(let i=0;i<lines.length;i++){
  const line=lines[i];if(!line.trim()){flush();continue;}let b={id:uid(),type:'paragraph',text:line};let m;
  if(line.startsWith('```')){flush();b.type='code';b.language=line.slice(3);let body=[];while(++i<lines.length&&!lines[i].startsWith('```'))body.push(lines[i]);b.text=body.join('\n');}
  else if(m=line.match(/^(#{1,3}) (.*)$/)){b.type=`h${m[1].length}`;b.text=m[2];}
  else if(m=line.match(/^- \[([ xX])\] (.*)$/)){b.type='todo';b.checked=m[1]!==' ';b.text=m[2];}
  else if(m=line.match(/^[-*] (.*)$/)){b.type='bullet';b.text=m[1];}
  else if(m=line.match(/^\d+\. (.*)$/)){b.type='number';b.text=m[1];}
  else if(line.startsWith('> ')){b.type='quote';b.text=line.slice(2);}
  else if(line.trim()==='---'){b.type='divider';b.text='';}
  else{para.push(line);continue;}flush();blocks.push(b);
 }flush();return blocks;
}
export function toMarkdown(blocks) {
 return blocks.map(b=>{
  if(/^h[123]$/.test(b.type))return '#'.repeat(Number(b.type[1]))+' '+b.text;
  if(b.type==='todo')return `- [${b.checked?'x':' '}] ${b.text}`;
  if(b.type==='bullet')return '- '+b.text;if(b.type==='number')return '1. '+b.text;
  if(['quote','callout'].includes(b.type))return '> '+b.text;
  if(b.type==='code')return '```'+(b.language||'')+'\n'+b.text+'\n```';
  if(b.type==='divider')return '---';return b.text;
 }).join('\n\n');
}
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
