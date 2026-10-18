import test from 'node:test';
import assert from 'node:assert/strict';
import { parseMarkdown,inlineMarkdown,toMarkdown } from '../ui/web/core.mjs';
import { tableParse } from '../ui/web/database.mjs';
const helpers=await import('../ui/web/markdown.mjs').catch(()=>({}));
const document=['# 清理指南','','## 1. 构建缓存','','```bash','docker builder prune -f # 清理没有用的缓存','docker builder prune -af','```','','正文中使用 `buildx`，然后继续普通文字。','','## 2. 镜像','','~~~bash','docker image prune -f','~~~','','**注意**：这里只保存示例命令，不执行命令。'].join('\n');
test('整篇文档正确分隔两种代码围栏与后续正文',()=>{
 const blocks=parseMarkdown(document),code=blocks.filter(b=>b.type==='code');assert.equal(code.length,2);assert.equal(code[0].language,'bash');assert.equal(code[0].text,'docker builder prune -f # 清理没有用的缓存\ndocker builder prune -af');assert.equal(code[1].text,'docker image prune -f');assert.equal(blocks.at(-1).type,'paragraph');assert.match(blocks.at(-1).text,/注意/);
});
test('行内代码隔离链接和强调，转义及多反引号不串入正文',()=>{
 const html=inlineMarkdown('`**原文** [地址](https://example.com)` 后面的 __粗体__、_斜体_ 和 ~~删除~~。');assert.match(html,/<code>\*\*原文\*\* \[地址\]\(https:\/\/example.com\)<\/code>/);assert.match(html,/<strong>粗体<\/strong>/);assert.match(html,/<em>斜体<\/em>/);assert.match(html,/<s>删除<\/s>/);
 assert.match(inlineMarkdown('`` const x = `value`; ``'),/<code>const x = `value`;<\/code>/);assert.ok(!inlineMarkdown('\\*不是斜体\\*').includes('<em>'));
});
test('链接括号、页面链接和附件在代码隔离后保持可用且不执行网页',()=>{
 const html=inlineMarkdown('`[[页面]]` [[页面]] [链接](https://example.com/a_(b)?x=1&y=2) [附件](asset:demo.pdf)',[{id:'p1',title:'页面'}]);assert.equal((html.match(/href="page:p1"/g)||[]).length,1);assert.match(html,/href="https:\/\/example.com\/a_\(b\)\?x=1&amp;y=2"/);assert.match(html,/data-asset="demo.pdf"/);
 const unsafe=inlineMarkdown('<img src=x onerror=alert(1)> [危险](javascript:alert(1)) ![危险](data:image/svg+xml,evil)');assert.ok(!/<img|href="javascript:|src="data:/i.test(unsafe));
});
test('旧版长段落也能显示完整文档且代码不污染后文',()=>{
 assert.equal(typeof helpers.renderMarkdown,'function','尚未实现完整文档渲染');const html=helpers.renderMarkdown(document);assert.equal((html.match(/<pre/g)||[]).length,2);assert.match(html,/<h2>2\. 镜像<\/h2>/);assert.match(html,/<strong>注意<\/strong>/);assert.ok(!html.includes('```bash'));assert.match(html,/<code class="language-bash">docker builder prune -f/);
});
test('六级标题、嵌套列表、起始序号及待办被保留',()=>{
 assert.equal(parseMarkdown('###### 六级标题')[0].type,'h6');const blocks=parseMarkdown('3. 第一项\n4. 第二项\n   - 子项\n     - 孙项\n\n- [x] 已完成\n+ [ ] 未完成');assert.deepEqual(blocks.map(b=>b.type),['number','number','bullet','bullet','todo','todo']);assert.equal(blocks[0].order,3);assert.equal(blocks[1].order,4);assert.equal(blocks[2].depth,1);assert.equal(blocks[3].depth,2);assert.equal(blocks[4].checked,true);assert.equal(blocks[5].checked,false);
 const round=toMarkdown(blocks);assert.match(round,/3\. 第一项/);assert.match(round,/4\. 第二项/);assert.equal(parseMarkdown(round)[3].depth,2);
});
test('引用跨行与层级、分隔线、缩进代码和表格不会丢失',()=>{
 const blocks=parseMarkdown('> 第一行\n> 第二行\n\n***\n\n    const value = 1;\n\n| 名称 | 描述 |\n| --- | --- |\n| 甲 | a\\|b |');assert.equal(blocks[0].type,'quote');assert.equal(blocks[0].text,'第一行\n第二行');assert.equal(blocks[1].type,'divider');assert.equal(blocks[2].type,'code');assert.equal(blocks[3].type,'table');assert.deepEqual(tableParse(blocks[3].text),[['名称','描述'],['甲','a|b']]);assert.match(toMarkdown(blocks),/> 第一行\n> 第二行/);
});
test('块粘贴保留选择区两侧正文及唯一标识，且不修改输入对象',()=>{
 assert.equal(typeof helpers.prepareMarkdownPaste,'function','尚未实现结构化粘贴');const original={id:'original',type:'paragraph',text:'前文选区后文',backgroundColor:'purple'};const result=helpers.prepareMarkdownPaste(original,'## 标题\n\n```bash\necho ok\n```',2,4);assert.deepEqual(result.map(b=>b.type),['paragraph','h2','code','paragraph']);assert.equal(result[0].text,'前文');assert.equal(result.at(-1).text,'后文');assert.equal(result[0].id,'original');assert.equal(new Set(result.map(b=>b.id)).size,result.length);assert.equal(original.text,'前文选区后文');
});
test('代码内粘贴和普通文字沿用原始插入，替换全文不丢旧块标识',()=>{
 assert.equal(typeof helpers.prepareMarkdownPaste,'function');assert.equal(helpers.prepareMarkdownPaste({id:'code',type:'code',text:'literal'},'# 原文\n\n**原样**',0,0),null);assert.equal(helpers.prepareMarkdownPaste({id:'text',type:'paragraph',text:'前后'},'普通文字',1,1),null);const blocks=helpers.prepareMarkdownPaste({id:'old',type:'paragraph',text:'整段旧文'},'# 新标题\n\n正文',0,4);assert.equal(blocks.length,2);assert.equal(blocks[0].id,'old');
});
test('导出围栏长于代码中的反引号并保留所有代码行',()=>{
 const text='first\n```\nsecond\n````\nlast';const exported=toMarkdown([{id:'code',type:'code',language:'text',text}]);assert.match(exported,/^`{5}text\n/);assert.equal(parseMarkdown(exported)[0].text,text);
});
test('复杂引用列表和长编号保持整篇原文与渲染结构',()=>{
 for(const source of ['- 父项\n  > 引用正文\n  >\n  > - 引用中的子项\n\n- 第二项','> - 父项\n>   - 子项','1000. 父项\n      - 子项','99. 前项\n100. 父项\n     - 子项','1000. 父项\n\n      续段','> 第一段\n>\n> 第二段','> 一级\n>> 二级']){
  const blocks=parseMarkdown(source);assert.equal(toMarkdown(blocks),source);assert.equal(helpers.renderMarkdown(toMarkdown(blocks)),helpers.renderMarkdown(source));
 }
});
test('整块替换不继承旧待办状态或引用缩进',()=>{
 for(const original of [{id:'quote',type:'quote',text:'原文',quoteDepth:2,depth:1},{id:'todo',type:'todo',text:'原文',checked:true,listType:'number',order:6}]){
  const blocks=helpers.prepareMarkdownPaste(original,'# 新标题\n\n正文',0,2);assert.equal(blocks[0].id,original.id);for(const key of ['checked','quoteDepth','depth','listType','order'])assert.equal(blocks[0][key],undefined);
 }
});
test('旧长段落围栏内粘贴保持原文插入，围栏外编辑不拆坏上下文',()=>{
 const original={id:'legacy',type:'paragraph',text:'# 文档\n\n```text\n原文\n```\n\n结尾'},position=original.text.indexOf('原文');assert.equal(helpers.prepareMarkdownPaste(original,'# 代码原文\n\n内容',position,position),null);
 const blocks=helpers.prepareMarkdownPaste(original,'## 新章节\n\n正文',original.text.length,original.text.length);assert.equal(blocks.length,1);assert.equal(blocks[0].text,original.text+'## 新章节\n\n正文');
});
test('引用式链接文档中插入章节仍保留整篇定义上下文',()=>{
 const original={id:'references',type:'paragraph',text:'原文 [地址][ref]\n\n[ref]: https://example.com',markdownDocument:true},position=original.text.indexOf('\n'),value='\n\n# 新章节\n\n正文\n\n';
 const blocks=helpers.prepareMarkdownPaste(original,value,position,position);assert.equal(blocks.length,1);assert.match(helpers.renderMarkdown(blocks[0].text),/href="https:\/\/example.com"/);assert.equal(blocks[0].text,original.text.slice(0,position)+value+original.text.slice(position));
});
test('同一引用内的不同内容块保持引用组边界',()=>{
 for(const source of ['> # 标题\n>\n> 段落','> ```text\n> literal\n> ```\n>\n> 说明','> | 名称 | 值 |\n> | --- | --- |\n> | 甲 | 乙 |\n>\n> 说明'])assert.equal(helpers.renderMarkdown(toMarkdown(parseMarkdown(source))),helpers.renderMarkdown(source));
});
test('嵌套段落粘贴的后缀是同级正文，不携带原来的绝对缩进',async()=>{const {replaceBlockTree,blockTree}=await import('../ui/web/block-tree.mjs'),original={id:'b',type:'paragraph',text:'前后',parentId:'a',depth:1},replacement=helpers.prepareMarkdownPaste(original,'- 甲\n- 乙',1,1);const result=replaceBlockTree([{id:'a',type:'toggle',text:'父'},original,{id:'c',type:'paragraph',text:'原子内容',parentId:'b',depth:2}],'b',replacement),tail=result.find(b=>b.text==='后');assert.equal(tail.parentId,'a');assert.equal(result.find(b=>b.id==='c').parentId,tail.id);assert.equal(blockTree(result).byId.get(tail.id).depth,1);});
test('整篇旧文档与围栏代码保持源码键盘编辑，普通段落允许拆块',()=>{assert.equal(typeof helpers.isMarkdownSourceBlock,'function');assert.equal(helpers.isMarkdownSourceBlock({type:'paragraph',text:'```js\nconst n = 12;\n```'}),true);assert.equal(helpers.isMarkdownSourceBlock({type:'paragraph',text:'# 旧章节\n\n正文',markdownDocument:true}),true);assert.equal(helpers.isMarkdownSourceBlock({type:'paragraph',text:'普通 `单行代码` 正文'}),false);});
test('无整篇标记的旧段落引用定义仍按完整源码编辑',()=>{assert.equal(helpers.isMarkdownSourceBlock({type:'paragraph',text:'原文 [地址][ref]\n\n[ref]: https://example.com'}),true);});
