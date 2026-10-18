import { uid,clone } from './text.mjs';
import { t } from './i18n.mjs';
export const canNestBlock=block=>['paragraph','bullet','number','todo','quote','callout','toggle','h1','h2','h3','h4','h5','h6'].includes(block?.type);
export function blockTree(blocks){
 const roots=[],byId=new Map(),entries=[],stack=[];for(const [index,block] of blocks.entries()){
  if(!block||!block.id||byId.has(block.id)||Object.hasOwn(block,'collapsed')&&typeof block.collapsed!=='boolean'||Object.hasOwn(block,'depth')&&(!Number.isInteger(block.depth)||block.depth<0||block.depth>64))throw new Error(t('invalidBlockTree'));
  let parent=null;if(Object.hasOwn(block,'parentId')){if(typeof block.parentId!=='string')throw new Error(t('invalidBlockTree'));if(block.parentId){parent=byId.get(block.parentId);if(!parent||!stack.includes(parent)||!canNestBlock(parent.block))throw new Error(t('invalidBlockTree'));}}
  else if(Number.isInteger(block.depth)&&block.depth>0){parent=stack[Math.min(stack.length,block.depth)-1]||null;while(parent&&!canNestBlock(parent.block))parent=parent.parent;}
  while(stack.length&&stack.at(-1)!==parent){stack.pop().end=index;}
  const entry={block,index,end:blocks.length,parent,parentId:parent?.block.id||'',depth:parent?parent.depth+1:0,children:[]};if(entry.depth>64)throw new Error(t('blockDepthLimit'));
  (parent?parent.children:roots).push(entry);byId.set(block.id,entry);entries.push(entry);stack.push(entry);
 }return {roots,byId,entries};
}
export function normalizeBlockTree(blocks){const forest=blockTree(blocks),result=forest.entries.map(entry=>({...clone(entry.block),parentId:entry.parentId,depth:entry.depth}));return result;}
function normalized(blocks){const result=normalizeBlockTree(blocks);return {blocks:result,forest:blockTree(result)};}
function finish(blocks,previous=[]){
 const isNumber=b=>b.type==='number'||b.type==='todo'&&b.listType==='number',origins=new Map();
 const remember=nodes=>{let start=null;for(const node of nodes){if(isNumber(node.block)){start??=Number.isInteger(node.block.order)&&node.block.order>0?node.block.order:1;origins.set(node.block.id,{parentId:node.parentId,start});}else start=null;remember(node.children);}};remember(blockTree(previous).roots);
 const renumber=nodes=>{for(let i=0;i<nodes.length;){if(!isNumber(nodes[i].block)){renumber(nodes[i++].children);continue;}let end=i;while(end<nodes.length&&isNumber(nodes[end].block))end++;const run=nodes.slice(i,end),origin=run.map(n=>origins.get(n.block.id)).find(o=>o&&o.parentId===nodes[i].parentId);let next=origin?.start??(run.some(n=>origins.has(n.block.id))?1:Number.isInteger(nodes[i].block.order)&&nodes[i].block.order>0?nodes[i].block.order:1);for(const node of run){node.block.order=next++;renumber(node.children);}i=end;}};renumber(blockTree(blocks).roots);return normalizeBlockTree(blocks);
}
export function moveBlockTree(source,id,targetId,position='before'){
 const {blocks,forest}=normalized(source),node=forest.byId.get(id),target=forest.byId.get(targetId);if(!node||!target||!['before','after','inside'].includes(position)||target.index>=node.index&&target.index<node.end||position==='inside'&&!canNestBlock(target.block))return source;
 const group=blocks.slice(node.index,node.end),remaining=blocks.filter((_,i)=>i<node.index||i>=node.end),next=blockTree(remaining).byId.get(targetId);group[0].parentId=position==='inside'?targetId:next.parentId;
 const index=position==='before'?next.index:position==='after'?next.end:next.end;
 if(position==='inside'&&next.block.type==='toggle')next.block.collapsed=false;remaining.splice(index,0,...group);return finish(remaining,source);
}
export function shiftBlockTree(blocks,id,direction){const forest=blockTree(blocks),node=forest.byId.get(id);if(!node)return blocks;const siblings=node.parent?.children||forest.roots,index=siblings.indexOf(node),target=siblings[index+(direction<0?-1:1)];return target?moveBlockTree(blocks,id,target.block.id,direction<0?'before':'after'):blocks;}
export function indentBlockTree(blocks,id){const forest=blockTree(blocks),node=forest.byId.get(id);if(!node)return blocks;const siblings=node.parent?.children||forest.roots,previous=siblings[siblings.indexOf(node)-1];return previous&&canNestBlock(previous.block)?moveBlockTree(blocks,id,previous.block.id,'inside'):blocks;}
export function outdentBlockTree(blocks,id){const node=blockTree(blocks).byId.get(id);return node?.parent?moveBlockTree(blocks,id,node.parent.block.id,'after'):blocks;}
export function removeBlockTree(source,id){const {blocks,forest}=normalized(source),node=forest.byId.get(id);return node?finish(blocks.filter((_,index)=>index<node.index||index>=node.end),source):source;}
export function duplicateBlockTree(source,id){const {blocks,forest}=normalized(source),node=forest.byId.get(id);if(!node)return source;const group=clone(blocks.slice(node.index,node.end)),ids=new Map(group.map(b=>[b.id,uid()]));for(const block of group){block.id=ids.get(block.id);if(ids.has(block.parentId))block.parentId=ids.get(block.parentId);}blocks.splice(node.end,0,...group);return finish(blocks,source);}
export function insertBlockTree(source,block,id='',position='after'){
 const {blocks,forest}=normalized(source),node=forest.byId.get(id),copy=clone(block);if(position==='inside'&&(!node||!canNestBlock(node.block)))return source;
 copy.parentId=node?(position==='inside'?id:node.parentId):'';delete copy.depth;const index=node?(position==='inside'?node.index+1:position==='before'?node.index:node.end):blocks.length;if(position==='inside'&&node.block.type==='toggle')node.block.collapsed=false;blocks.splice(index,0,copy);return finish(blocks,source);
}
export function splitBlockTree(source,id,start,end=start){
 const node=blockTree(source).byId.get(id);if(!node||node.block.type==='code')return {blocks:source,focusId:id};const block=node.block,text=String(block.text||''),from=Math.max(0,Math.min(text.length,start)),to=Math.max(from,Math.min(text.length,end));
 if(!text&&['bullet','number','todo'].includes(block.type)){if(node.parent)return {blocks:outdentBlockTree(source,id),focusId:id};const blocks=normalizeBlockTree(source),current=blocks.find(b=>b.id===id);current.type='paragraph';delete current.checked;delete current.order;delete current.listType;return {blocks,focusId:id};}
 const blocks=normalizeBlockTree(source),current=blocks.find(b=>b.id===id);current.text=text.slice(0,from);const type=['bullet','number','todo'].includes(block.type)?block.type:'paragraph',next={id:uid(),type,text:text.slice(to)};if(type==='todo'){next.checked=false;if(block.listType)next.listType=block.listType;}if(block.order)next.order=block.order+1;
 return {blocks:insertBlockTree(blocks,next,id,block.type==='toggle'?'inside':'after'),focusId:next.id};
}
export function replaceBlockTree(source,id,replacement){
 const {blocks,forest}=normalized(source),node=forest.byId.get(id);if(!node||!replacement.length)return source;
 const local=clone(replacement);for(const block of local){delete block.parentId;if(block.id===id)delete block.depth;}
 const inserted=normalizeBlockTree(local);for(const block of inserted)if(!block.parentId)block.parentId=node.parentId;
 const children=blocks.slice(node.index+1,node.end);if(children.length){let owner=[...inserted].reverse().find(b=>b.parentId===node.parentId&&canNestBlock(b));if(!owner){owner={id:uid(),type:'paragraph',text:'',parentId:node.parentId};inserted.push(owner);}for(const child of children)if(child.parentId===id)child.parentId=owner.id;const subtree=blockTree(inserted.map(b=>({...b,parentId:b.parentId===node.parentId?'':b.parentId}))).byId.get(owner.id);inserted.splice(subtree.end,0,...children);}
 blocks.splice(node.index,node.end-node.index,...inserted);return finish(blocks,source);
}

export function renumberBlockTree(blocks,previous=blocks){return finish(normalizeBlockTree(blocks),previous);}
