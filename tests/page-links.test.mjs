import test from 'node:test';
import assert from 'node:assert/strict';
import { pageMenuItems } from '../ui/web/page-menu.mjs';
const m=await import('../ui/web/page-links.mjs').catch(()=>({}));
test('只有独立页面链接显示为页面条目，混排、代码及明确文字形式不转换',()=>{
 assert.equal(typeof m.pageLinkTarget,'function','页面条目识别尚未实现');
 const pages=[{id:'p1',title:'数据库',icon:'🗃️'},{id:'p2',title:'回收',deleted:true}];
 assert.equal(m.pageLinkTarget({type:'paragraph',text:'[旧名称](page:p1)'},pages).id,'p1');
 assert.equal(m.pageLinkTarget({type:'paragraph',text:'[[数据库]]'},pages).id,'p1');
 for(const block of [{type:'paragraph',text:'参考 [数据库](page:p1)'},{type:'code',text:'[数据库](page:p1)'},{type:'paragraph',text:'[数据库](page:p1)',pagePresentation:'inline'},{type:'paragraph',text:'[回收](page:p2)'}])assert.equal(m.pageLinkTarget(block,pages),null);
});
test('页面条目使用当前标题、真实图标与安全颜色，转义而不执行内容',()=>{
 assert.equal(typeof m.renderPageLink,'function','页面条目渲染尚未实现');
 const html=m.renderPageLink({id:'p1',title:'<标题>',icon:'asset:abc.png'},{id:'b1',textColor:'green',backgroundColor:'purple'});
 assert.match(html,/data-asset="abc.png"/);assert.match(html,/&lt;标题&gt;/);assert.match(html,/text-green/);assert.match(html,/background-purple/);assert.match(html,/page:p1/);
 assert.ok(!m.renderPageLink({id:'p1',title:'标题',icon:'📄'},{textColor:'red;position:fixed',backgroundColor:'url(x)'}).includes('position:fixed'));
});
test('页面菜单包含搜索所需的分组、颜色和打开方式，正文链接支持转换',()=>{
 const items=pageMenuItems({title:'子页'},{block:true});
 for(const action of ['pageMenuConvert','pageMenuColor','pageMenuIcon','pageMenuTab','pageMenuWindow','pageMenuPeek','pageMenuCopyLink'])assert.ok(items.some(i=>i.action===action),`缺少操作 ${action}`);
 assert.ok(items.some(i=>i.separator));assert.equal(pageMenuItems({locked:true},{block:true}).find(i=>i.action==='pageMenuIcon').disabled,true);
});
