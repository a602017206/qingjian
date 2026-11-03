import test from 'node:test';
import assert from 'node:assert/strict';
import {mindmapHTML} from '../ui/web/mindmap-view.mjs';
import {createMindmap,addChild} from '../ui/web/mindmap-model.mjs';
test('预览绘制节点并转义用户输入，折叠时隐藏子节点',()=>{let d=addChild(createMindmap('root','<script>'),'root','child','子节点');let html=mindmapHTML(d,{readonly:true});assert.match(html,/&lt;script&gt;/);assert.match(html,/子节点/);assert.doesNotMatch(html,/<script>/);d.nodes[0].collapsed=true;html=mindmapHTML(d,{readonly:true});assert.doesNotMatch(html,/子节点/);});
test('允许的两千层节点不会耗尽调用栈或丢失内容',()=>{const nodes=Array.from({length:2000},(_,i)=>({id:'n'+i,parentId:i?'n'+(i-1):'',text:'节点'+i,order:0}));const html=mindmapHTML({nodes},{readonly:true});assert.match(html,/节点1999/);assert.equal((html.match(/role="treeitem"/g)||[]).length,2000);});
test('可编辑节点的页面关联提供安全跳转入口',()=>{const map=createMindmap('root','主题');map.nodes[0].pageId='page1';const html=mindmapHTML(map,{pages:[{id:'page1',title:'关联页面'}]});assert.match(html,/data-action="page"/);assert.match(html,/data-id="page1"/);});
