import {extractPageReferences} from './markdown.mjs';
const parsedBlocks=new WeakMap();
function blockReferences(block){const cached=parsedBlocks.get(block);if(cached&&cached.text===block.text&&cached.type===block.type&&cached.markdownDocument===block.markdownDocument)return cached.refs;const refs=extractPageReferences(block);parsedBlocks.set(block,{text:block.text,type:block.type,markdownDocument:block.markdownDocument,refs});return refs;}

// 索引只在当前调用中派生，不缓存到仓库或跨工作区共享。
export function buildGraph(pages){
 const active=pages.filter(p=>!p.deleted&&!p.isTemplate),byId=new Map(active.map(p=>[p.id,p])),titles=new Map(),edges=new Map(),issues=[];
 for(const p of active){const group=titles.get(p.title)||[];group.push(p);titles.set(p.title,group);}
 function add(source,target,kind,location){const key=JSON.stringify([source,target,kind]);let edge=edges.get(key);if(!edge){edge={source,target,kind,locations:[]};edges.set(key,edge);}edge.locations.push(location);}
 function reference(source,ref,location,kind='reference'){
  const matches=ref.kind==='id'?(byId.has(ref.value)?[byId.get(ref.value)]:[]):(titles.get(ref.value)||[]);
  if(matches.length===1)add(source,matches[0].id,kind,{...location,match:ref.kind});else issues.push({source,reason:matches.length?'ambiguous':'missing',value:ref.value,...location});
 }
 for(const p of active){
  if(p.parentId&&byId.has(p.parentId))add(p.parentId,p.id,'hierarchy',{pageId:p.id});
  for(const b of p.blocks||[])for(const ref of blockReferences(b))reference(p.id,ref,{blockId:b.id,text:String(b.text||'').slice(0,240)});
  if(p.diagram?.schemaVersion===1||p.diagram?.schemaVersion===2&&p.diagram?.mode==='freeform')for(const node of p.diagram.nodes||[]){if(node.pageId)reference(p.id,{kind:'id',value:node.pageId},{nodeId:node.id,text:node.text||''},'diagram');else if(node.unresolvedPageId)issues.push({source:p.id,reason:'missing',value:node.unresolvedPageId,nodeId:node.id,text:node.text||''});}
 }
 return {nodes:active.map(p=>({id:p.id,title:p.title,icon:p.icon,tags:p.tags||[]})),edges:[...edges.values()],issues};
}
export function selectGraph(graph,{center='',hops=1,kinds=['reference','diagram'],query='',tag='',limit=200}={}){
 const types=new Set(kinds),allowed=graph.edges.filter(e=>types.has(e.kind)),byId=new Map(graph.nodes.map(n=>[n.id,n]));let candidate=new Set(byId.keys());
 if(center){candidate=new Set(byId.has(center)?[center]:[]);let frontier=new Set(candidate);for(let i=0;i<Math.min(2,Math.max(1,hops));i++){const next=new Set();for(const e of allowed){if(frontier.has(e.source)&&!candidate.has(e.target))next.add(e.target);if(frontier.has(e.target)&&!candidate.has(e.source))next.add(e.source);}for(const id of next)candidate.add(id);frontier=next;}}
 const term=query.trim().toLocaleLowerCase();const nodes=graph.nodes.filter(n=>candidate.has(n.id)&&(!term||n.title.toLocaleLowerCase().includes(term))&&(!tag||n.tags.includes(tag)));const total=nodes.length,ordered=center&&nodes.some(n=>n.id===center)?[nodes.find(n=>n.id===center),...nodes.filter(n=>n.id!==center)]:nodes,shown=ordered.slice(0,Math.max(1,limit)),ids=new Set(shown.map(n=>n.id));return {nodes:shown,edges:allowed.filter(e=>ids.has(e.source)&&ids.has(e.target)),issues:graph.issues.filter(i=>ids.has(i.source)),total,truncated:total>shown.length};
}
export function createGraphCache(){let snapshot=null;return {clear(){snapshot=null;},get(workspaceId,pages){if(snapshot&&snapshot.workspaceId===workspaceId&&snapshot.pages.length===pages.length&&snapshot.pages.every((p,i)=>p===pages[i]))return snapshot.graph;const graph=buildGraph(pages);snapshot={workspaceId,pages:pages.slice(),graph};return graph;}};}
