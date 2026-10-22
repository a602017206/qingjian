import test from 'node:test';
import assert from 'node:assert/strict';
import * as markdown from '../ui/web/markdown.mjs';
test('普通内容块预览保留每行缩进空行及制表符',()=>{assert.equal(typeof markdown.renderBlockText,'function');assert.equal(markdown.renderBlockText({type:'paragraph',text:'电脑\n  桌面\n\t演示\n\n结尾'}),'电脑<br>  桌面<br>\t演示<br><br>结尾');});
test('普通内容预览仍安全处理格式与链接，复杂文档保持完整结构',()=>{assert.equal(typeof markdown.renderBlockText,'function');assert.equal(markdown.renderBlockText({type:'paragraph',text:'  **加粗**\n<script>内容</script>'}),'  <strong>加粗</strong><br>&lt;script&gt;内容&lt;/script&gt;');assert.match(markdown.renderBlockText({type:'paragraph',text:'# 标题\n\n```js\nconst a = 1;\n```',markdownDocument:true}),/<h1>标题<\/h1>/);assert.match(markdown.renderBlockText({type:'code',text:'<a>'}),/&lt;a&gt;/);});
test('普通内容末尾换行保留最后一行的排版占位',()=>{assert.match(markdown.renderBlockText({type:'paragraph',text:'末尾\n'}),/末尾<br><span class="block-trailing-line"/);});
