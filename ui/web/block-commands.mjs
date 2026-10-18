import { uid } from './text.mjs';
import { t } from './i18n.mjs';
import { normalizeBlockTree,renumberBlockTree,blockTree,canNestBlock,insertBlockTree } from './block-tree.mjs';
import { tableFromBlock } from './database.mjs';
export function applyBlockCommand(source,{type,mode,blockId,trigger,value,assetText}){
 let blocks=normalizeBlockTree(source),block=blocks.find(b=>b.id===blockId),insert=mode==='insert',prefix='',suffix='';
 if(mode!=='insert'&&!block)return {blocks:source,focusId:''};
 if(mode==='slash'){prefix=value.slice(0,trigger.start);suffix=value.slice(trigger.end);block.text=prefix;insert=!!prefix.trim();}
 const atomic=['image','divider'].includes(type);if(atomic&&block&&(block.text||blockTree(blocks).byId.get(blockId).children.length))insert=true;
 if(insert||!block)block={id:uid(),type:'paragraph',text:mode==='slash'&&!atomic?suffix:''};else if(mode==='slash')block.text=prefix+(atomic?'':suffix);
 if(!insert&&blockTree(blocks).byId.get(block.id)?.children.length&&!canNestBlock({type}))throw new Error(t('cannotNestBlock'));
 if(type!=='todo')delete block.checked;if(!['todo','number'].includes(type)){delete block.order;delete block.listType;}if(type!=='paragraph')delete block.markdownDocument;
 if(type==='table')block.text=tableFromBlock(block);block.type=type;if(type==='toggle')block.collapsed=false;if(type==='todo')block.checked=!!block.checked;
 if(type==='image'){block.type='paragraph';block.text=assetText;}if(type==='bookmark'){block.type='paragraph';block.text=block.text||t('bookmarkExample');}
 if(insert||!blocks.some(b=>b.id===block.id))blocks=insertBlockTree(blocks,block,blockId);
 let focusId=['divider','image','table'].includes(type)?'':block.id;
 if(mode==='slash'&&atomic&&suffix){const tail={id:uid(),type:'paragraph',text:suffix};blocks=insertBlockTree(blocks,tail,block.id);focusId=tail.id;}
 return {blocks:renumberBlockTree(blocks,source),focusId};
}
