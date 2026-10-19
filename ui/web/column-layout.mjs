import { uid } from './text.mjs';
import { t } from './i18n.mjs';
import { blockTree,normalizeBlockTree,renumberBlockTree } from './block-tree.mjs';
import { selectedBlockRoots,selectedBlockCoverage } from './block-batch.mjs';
const container=(type,parentId,width)=>({id:uid(),type,text:'',parentId,...(width===undefined?{}:{width})});
function countColumns(count){if(!Number.isInteger(count)||count<2||count>6)throw new Error(t('invalidColumns'));}
export function createBlockColumns(source,ids,count=2){
 countColumns(count);const roots=selectedBlockRoots(source,ids),blocks=normalizeBlockTree(source),forest=blockTree(blocks),nodes=roots.map(id=>forest.byId.get(id));if(!nodes.length)return source;
 if(nodes.some((node,i)=>node.parentId!==nodes[0].parentId||i&&nodes[i-1].end!==node.index))throw new Error(t('columnsNeedSiblings'));
 const group=container('columns',nodes[0].parentId),cols=Array.from({length:count},()=>container('column',group.id,1/count)),result=[group];
 for(const [index,col] of cols.entries()){result.push(col);for(const [i,node] of nodes.entries())if(Math.min(count-1,Math.floor(i*count/nodes.length))===index){const subtree=blocks.slice(node.index,node.end);subtree[0].parentId=col.id;result.push(...subtree);}}
 blocks.splice(nodes[0].index,nodes.at(-1).end-nodes[0].index,...result);return renumberBlockTree(blocks,source);
}
export function insertBlockColumns(source,afterId='',count=2){countColumns(count);const blocks=normalizeBlockTree(source),node=blockTree(blocks).byId.get(afterId),group=container('columns',node?.parentId||''),result=[group];for(let i=0;i<count;i++)result.push(container('column',group.id,1/count));blocks.splice(node?.end??blocks.length,0,...result);return renumberBlockTree(blocks,source);}
export function dropBlockColumns(source,ids,targetId,side){
 if(!['left','right'].includes(side))return source;const roots=selectedBlockRoots(source,ids),covered=selectedBlockCoverage(source,roots),forest=blockTree(source),target=forest.byId.get(targetId);if(!roots.length||!target||covered.has(targetId)||target.block.type==='column')return source;
 const blocks=normalizeBlockTree(source),moving=blocks.filter(b=>covered.has(b.id)),remaining=blocks.filter(b=>!covered.has(b.id)),tree=blockTree(remaining),next=tree.byId.get(targetId);let col=next.parent;while(col&&col.block.type!=='column')col=col.parent;
 if(col){const owner=col.parent;if(owner.children.length>=6)throw new Error(t('columnLimit'));const added=container('column',owner.block.id,owner.children.reduce((sum,n)=>sum+(n.block.width||1),0)/owner.children.length);for(const b of moving)if(roots.includes(b.id))b.parentId=added.id;remaining.splice(side==='left'?col.index:col.end,0,added,...moving);return renumberBlockTree(remaining,source);}
 const group=container('columns',next.parentId),left=container('column',group.id,.5),right=container('column',group.id,.5),original=remaining.splice(next.index,next.end-next.index);original[0].parentId=side==='left'?right.id:left.id;for(const b of moving)if(roots.includes(b.id))b.parentId=side==='left'?left.id:right.id;
 remaining.splice(next.index,0,group,left,...(side==='left'?moving:original),right,...(side==='left'?original:moving));return renumberBlockTree(remaining,source);
}
export function flattenBlockColumns(source,id){const blocks=normalizeBlockTree(source),node=blockTree(blocks).byId.get(id);if(node?.block.type!=='columns')return source;const content=[];for(const col of node.children)for(const child of col.children){const group=blocks.slice(child.index,child.end);group[0].parentId=node.parentId;content.push(...group);}blocks.splice(node.index,node.end-node.index,...content);return renumberBlockTree(blocks,source);}
export function equalizeBlockColumns(source,id){const blocks=normalizeBlockTree(source),node=blockTree(blocks).byId.get(id);if(node?.block.type!=='columns')return source;for(const col of node.children)col.block.width=1/node.children.length;return blocks;}
export function resizeBlockColumns(source,id,leftId,rightId,value){
 if(!Number.isFinite(value))throw new Error(t('invalidColumnWidth'));const blocks=normalizeBlockTree(source),node=blockTree(blocks).byId.get(id);if(node?.block.type!=='columns')throw new Error(t('invalidColumns'));const cols=node.children.map(n=>n.block),index=cols.findIndex(b=>b.id===leftId);if(index<0||cols[index+1]?.id!==rightId)throw new Error(t('invalidColumns'));
 const total=cols.reduce((sum,b)=>sum+(b.width||1),0);for(const col of cols)col.width=(col.width||1)/total;const pair=cols[index].width+cols[index+1].width,min=Math.min(.08,pair*.25),left=Math.max(min,Math.min(pair-min,value));cols[index].width=left;cols[index+1].width=pair-left;return blocks;
}
export function columnDropPosition(rect,x,y,canNest,base){const fraction=(y-rect.top)/Math.max(1,rect.height);if(rect.width>=160&&fraction>.15&&fraction<.85){if(x<rect.left+22)return 'left';if(x>rect.right-22)return 'right';}return base(rect,x,y,canNest);}
