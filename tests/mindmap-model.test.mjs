import test from 'node:test';
import assert from 'node:assert/strict';
import {createMindmap,addChild,removeBranch,moveNode} from '../ui/web/mindmap-model.mjs';

test('思维导图能添加和删除分支，不能移到自身后代',()=>{const map=createMindmap('r','根');const one=addChild(map,'r','a');const two=addChild(one,'a','b');assert.equal(two.nodes.length,3);assert.throws(()=>moveNode(two,'a','b'));assert.deepEqual(removeBranch(two,'a').nodes.map(n=>n.id),['r']);assert.equal(map.nodes.length,1);});

test('新建同级并调整顺序，导出包含折叠分支和页面引用', async()=>{
 const {addSibling,shiftSibling,mindmapMarkdown}=await import('../ui/web/mindmap-model.mjs');
 let m=addChild(createMindmap('r','根'),'r','a','甲');m=addSibling(m,'a','b','乙');m.nodes.find(n=>n.id==='a').pageId='page1';m=addChild(m,'a','c','子');m.nodes.find(n=>n.id==='a').collapsed=true;
 const shifted=shiftSibling(m,'b',-1);assert.deepEqual(shifted.nodes.filter(n=>n.parentId==='r').sort((a,b)=>a.order-b.order).map(n=>n.id),['b','a']);
 assert.equal(mindmapMarkdown(shifted),'- 根\n  - 乙\n  - [甲](page:page1)\n    - 子');
});

test('复制分支重新生成节点标识并保留页面关联',async()=>{const {duplicateBranch}=await import('../ui/web/mindmap-model.mjs');let m=addChild(createMindmap('r','根'),'r','a','甲');m=addChild(m,'a','b','乙');m.nodes[1].pageId='page1';let i=0;const next=duplicateBranch(m,'a',()=>`new${++i}`);assert.equal(next.nodes.length,5);assert.equal(next.nodes.find(n=>n.id==='new1').pageId,'page1');assert.equal(next.nodes.find(n=>n.id==='new2').parentId,'new1');assert.equal(m.nodes.length,3);});
