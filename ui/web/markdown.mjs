import MarkdownIt from 'markdown-it';
import { uid,escapeHTML as e,safeURL } from './text.mjs';
import { tableSerialize } from './database.mjs';
import { t } from './i18n.mjs';
import { blockTree } from './block-tree.mjs';

const md=new MarkdownIt({html:false,breaks:true,linkify:true,typographer:false});
md.validateLink=url=>!!safeURL(url);
md.inline.ruler.before('link','page_reference',(state,silent)=>{
 const start=state.pos;if(state.linkLevel||state.src.slice(start,start+2)!=='[[')return false;
 const end=state.src.indexOf(']]',start+2);if(end<0)return false;const title=state.src.slice(start+2,end);if(!title||title.includes('\n'))return false;
 if(!silent){const token=state.push('page_reference','',0);token.content=title;}state.pos=end+2;return true;
});
md.renderer.rules.page_reference=(tokens,index,options,env)=>{const title=tokens[index].content,page=env.pages?.find(p=>!p.deleted&&p.title===title),href=page&&safeURL('page:'+page.id);return href?`<a href="${e(href)}" class="page-link">↗ ${e(title)}</a>`:`<span class="unresolved-link">${e(title)}</span>`;};
md.renderer.rules.link_open=(tokens,index,options,env,self)=>{
 const token=tokens[index],href=token.attrGet('href')||'';
 if(href.startsWith('asset:')){let label='';for(let i=index+1;i<tokens.length&&tokens[i].type!=='link_close';i++)if(['text','code_inline'].includes(tokens[i].type))label+=tokens[i].content;return `<span class="asset-link" data-asset="${e(href.slice(6))}" data-label="${e(label)}">`;}
 if(href.startsWith('http')){token.attrSet('target','_blank');token.attrSet('rel','noopener noreferrer');}return self.renderToken(tokens,index,options);
};
md.renderer.rules.link_close=(tokens,index,options,env,self)=>{let nesting=0;for(let i=index-1;i>=0;i--){if(tokens[i].type==='link_close')nesting++;if(tokens[i].type==='link_open'){if(nesting)nesting--;else return tokens[i].attrGet('href')?.startsWith('asset:')?'</span>':self.renderToken(tokens,index,options);}}return '</a>';};
md.renderer.rules.image=(tokens,index)=>{const token=tokens[index],src=token.attrGet('src')||'',label=token.content||t('remoteImage');return src.startsWith('asset:')?`<span class="asset-preview" data-asset="${e(src.slice(6))}"></span>`:`<span class="external-image">${e(label)} <a href="${e(src)}" target="_blank" rel="noopener noreferrer">↗</a></span>`;};
md.core.ruler.after('inline','task_markers',state=>{for(let i=2;i<state.tokens.length;i++){const token=state.tokens[i];if(token.type!=='inline'||state.tokens[i-1].type!=='paragraph_open'||state.tokens[i-2].type!=='list_item_open')continue;const first=token.children?.[0],match=first?.type==='text'&&first.content.match(/^\[([ xX])\]\s+/);if(match){first.content=first.content.slice(match[0].length);const marker=new state.Token('task_marker','',0);marker.meta={checked:match[1]!==' '};token.children.unshift(marker);}}});
md.renderer.rules.task_marker=(tokens,index)=>`<input class="markdown-task" type="checkbox" disabled ${tokens[index].meta.checked?'checked':''} aria-label="${e(t('todo'))}">`;
export const inlineMarkdown=(value,pages=[])=>md.renderInline(String(value??''),{pages});
export const renderMarkdown=(value,pages=[])=>md.render(String(value??''),{pages});
const level=value=>Number.isInteger(value)&&value>0?Math.min(value,64):0;

export function parseMarkdown(value){
 const source=String(value??'').replaceAll('\r\n','\n').replaceAll('\r','\n'),env={},tokens=md.parse(source,env),blocks=[],lists=[],items=[];let quoteDepth=0;
 // 现有块模型无法表达容器交错及整篇链接定义时，保留原文和完整渲染。
 let quoteNesting=0,listNesting=0,quoteParagraphs=0,complex=false;const counters=[];
 for(const token of tokens){
  if(token.type==='blockquote_open'){if(quoteNesting||listNesting)complex=true;quoteNesting++;quoteParagraphs=0;}
  else if(token.type==='blockquote_close')quoteNesting--;
  else if(['bullet_list_open','ordered_list_open'].includes(token.type)){if(quoteNesting||Number(token.attrGet('start')||1)>=100)complex=true;listNesting++;counters.push(token.type==='ordered_list_open'?Number(token.attrGet('start')||1):null);}
  else if(['bullet_list_close','ordered_list_close'].includes(token.type)){listNesting--;counters.pop();}
  else if(token.type==='list_item_open'&&counters.at(-1)!==null){const index=counters.length-1;if(counters[index]++>=100)complex=true;}
  else if(quoteNesting&&(['heading_open','fence','code_block','table_open','hr'].includes(token.type)||token.type==='paragraph_open'&&++quoteParagraphs>1))complex=true;
 }
 if(complex||(env.references&&Object.keys(env.references).length))return [{id:uid(),type:'paragraph',text:source,markdownDocument:true}];
 const append=(type,text,extra={})=>{const block={id:uid(),type,text,...extra};if(quoteDepth)block.quoteDepth=quoteDepth;blocks.push(block);return block;};
 const ensureItem=()=>{const item=items.at(-1);if(item&&!item.emitted){append(item.type,'',{depth:lists.length-1,...(item.type==='number'?{order:item.order}:{})});item.emitted=true;}};
 for(let i=0;i<tokens.length;i++){
  const token=tokens[i];
  if(token.type==='blockquote_open'){quoteDepth++;continue;}if(token.type==='blockquote_close'){quoteDepth--;continue;}
  if(token.type==='bullet_list_open'||token.type==='ordered_list_open'){ensureItem();lists.push({type:token.type==='ordered_list_open'?'number':'bullet',next:Number(token.attrGet('start')||1)});continue;}
  if(token.type==='bullet_list_close'||token.type==='ordered_list_close'){lists.pop();continue;}
  if(token.type==='list_item_open'){const list=lists.at(-1);items.push({type:list.type,order:list.next++,emitted:false});continue;}if(token.type==='list_item_close'){ensureItem();items.pop();continue;}
  if(token.type==='table_open'){
   ensureItem();const rows=[];let row=[];while(++i<tokens.length&&tokens[i].type!=='table_close'){const cell=tokens[i];if(cell.type==='tr_open')row=[];if(cell.type==='inline')row.push(cell.content);if(cell.type==='tr_close')rows.push(row);}append('table',tableSerialize(rows),lists.length?{depth:lists.length}:{});continue;
  }
  if(token.type==='fence'||token.type==='code_block'){ensureItem();append('code',token.content.replace(/\n$/,''),{language:token.info||'',...(lists.length?{depth:lists.length}:{})});continue;}
  if(token.type==='hr'){ensureItem();append('divider','',lists.length?{depth:lists.length}:{});continue;}
  if(token.type!=='inline')continue;
  const item=items.at(-1),heading=tokens[i-1]?.type==='heading_open'?tokens[i-1].tag:null;
  if(heading){ensureItem();append(heading,token.content,lists.length?{depth:lists.length}:{});continue;}
  if(item&&!item.emitted){const task=token.content.match(/^\[([ xX])\]\s+([\s\S]*)$/);append(task?'todo':item.type,task?task[2]:token.content,{depth:lists.length-1,...(item.type==='number'?{order:item.order}:{}),...(task?{checked:task[1]!==' ',listType:item.type}:{})});item.emitted=true;}
  else append(quoteDepth?'quote':'paragraph',token.content,lists.length?{depth:lists.length,listContinuation:true}:{});
 }
 return blocks;
}
function flatMarkdown(blocks){
 return blocks.map(block=>{
  const body=String(block.text??''),depth=level(block.depth),indent='    '.repeat(depth),quote='> '.repeat(level(block.quoteDepth)||(['quote','callout'].includes(block.type)?1:0));let value=body,marker='';
  if(/^h[1-6]$/.test(block.type))value='#'.repeat(Number(block.type[1]))+' '+body;
  else if(block.type==='todo')marker=(block.listType==='number'?`${Number.isInteger(block.order)?block.order:1}. `:'- ')+`[${block.checked?'x':' '}] `;
  else if(block.type==='bullet')marker='- ';else if(block.type==='number')marker=`${Number.isInteger(block.order)?block.order:1}. `;
  else if(block.type==='code'){const info=String(block.language||'').replace(/[\r\n]/g,' '),char=info.includes('`')?'~':'`',runs=body.match(new RegExp(char+'+','g'))||[],fence=char.repeat(runs.reduce((max,run)=>Math.max(max,run.length+1),3));value=fence+info+'\n'+body+'\n'+fence;}
  else if(block.type==='divider')value='---';
  return value.split('\n').map((line,index)=>quote+indent+(marker?(index?' '.repeat(marker.length):marker):'')+line).join('\n');
 }).join('\n\n');
}
export function toMarkdown(blocks){
 if(!blocks.some(b=>b.parentId||b.type==='toggle'))return flatMarkdown(blocks);
 const forest=blockTree(blocks),prefixes=new Map();return forest.entries.map(entry=>{const block=entry.block,kind=block.type,listed=['bullet','number','todo'].includes(kind);let body=flatMarkdown([{...block,depth:0}]);
  if(kind==='toggle')body='- ▶ '+String(block.text||'').replaceAll('\n','\n  ');else if(!listed&&(entry.children.length||entry.parent&&kind==='paragraph'))body='- '+body.replaceAll('\n','\n  ');
  const prefix=entry.parent?prefixes.get(entry.parentId):'',width=kind==='number'||kind==='todo'&&block.listType==='number'?Math.max(4,String(block.order||1).length+2):4;prefixes.set(block.id,prefix+' '.repeat(width));return body.split('\n').map(line=>prefix+line).join('\n');
 }).join('\n\n');
}
export function prepareMarkdownPaste(block,value,start=0,end=start){
 if(block.type==='code'||typeof value!=='string'||!value)return null;
 const body=String(block.text||''),from=Math.max(0,Math.min(body.length,start)),to=Math.max(from,Math.min(body.length,end)),before=body.slice(0,from),after=body.slice(to);
 const existingEnv={},existing=block.type==='paragraph'?md.parse(body,existingEnv):[],line=body.slice(0,from).split('\n').length-1,endLine=body.slice(0,to).split('\n').length-1;
 if(existing.some(token=>['fence','code_block'].includes(token.type)&&token.map&&line>=token.map[0]+(token.type==='fence'?1:0)&&endLine<token.map[1]-(token.type==='fence'?1:0)))return null;
 const appearance=Object.fromEntries(['textColor','backgroundColor'].filter(key=>block[key]!==undefined).map(key=>[key,block[key]]));
 const pasted=parseMarkdown(value);if(!pasted.length||(pasted.length===1&&pasted[0].type==='paragraph'&&!pasted[0].markdownDocument))return null;
 if((before||after)&&(block.markdownDocument||existingEnv.references||existing.some(token=>['fence','heading_open','bullet_list_open','ordered_list_open','blockquote_open','table_open','hr','code_block'].includes(token.type))))return [{...appearance,id:block.id,type:'paragraph',text:before+value+after,markdownDocument:true}];
 const result=[];
 if(before)result.push({...block,text:before});
 result.push(...pasted);
 if(after){const tail={...block,id:uid(),text:after};delete tail.depth;delete tail.parentId;result.push(tail);}
 if(!before){result[0]={...appearance,...result[0],id:block.id};}
 return result;
}

export function isMarkdownSourceBlock(block){if(block.type==='code')return true;if(block.type!=='paragraph')return false;if(block.markdownDocument)return true;const env={},tokens=md.parse(String(block.text||''),env);return !!env.references||tokens.some(token=>['fence','code_block','heading_open','bullet_list_open','ordered_list_open','blockquote_open','table_open','hr'].includes(token.type));}
