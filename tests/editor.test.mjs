import test from 'node:test';
import assert from 'node:assert/strict';
import { parseMarkdown, toMarkdown, filterPages, safeURL, inlineMarkdown } from '../ui/web/core.mjs';

test('中文内容与待办及代码块往返', () => {
 const blocks=parseMarkdown('# 标题\n\n- [x] 完成事项\n\n```js\nconst x = 1;\n```');
 assert.equal(blocks[0].type,'h1');
 assert.equal(blocks[1].checked,true);
 assert.equal(blocks[2].type,'code');
 assert.match(toMarkdown(blocks),/- \[x\] 完成事项/);
});
test('搜索正文标签并排除回收站',()=>{
 assert.equal(filterPages([{title:'测试',tags:['设计'],blocks:[{text:'数据'}]},{title:'数据',deleted:true}], '数据').length,1);
 assert.equal(filterPages([{title:'测试',tags:['设计']}], '设计').length,1);
});
test('阻止活动链接并转义内嵌网页',()=>{
 assert.equal(safeURL('javascript:alert(1)'), '');
 assert.equal(safeURL('https://example.com'), 'https://example.com');
 assert.equal(safeURL('page:abc'), 'page:abc');
 assert.ok(!inlineMarkdown('<img src=x onerror=alert(1)>').includes('<img'));
 assert.ok(!inlineMarkdown('[点击](javascript:alert(1))').includes('href="javascript:'));
});
