import test from 'node:test';
import assert from 'node:assert/strict';
import { renderDatabase } from '../ui/web/database-ui.mjs';
import { computeCell, parseFormula, formatValue, readSelection, tableParse, tableSerialize, databaseCSV, calendarCells } from '../ui/web/database.mjs';
const page={id:'orders',database:{columns:[{id:'name',name:'商品',type:'text'},{id:'price',name:'单价',type:'number'},{id:'qty',name:'数量',type:'number'},{id:'total',name:'金额',type:'formula',formula:'prop("单价") * prop("数量")'},{id:'status',name:'说明',type:'formula',formula:'if(prop("金额") >= 100, "达标", "继续")'}],rows:[{id:'r1',values:{name:'笔记本',price:'25',qty:'4'}}]}};
test('图库使用关联记录名称，记录删除后显示中文提示',()=>{
 const target={id:'tasks',database:{columns:[{id:'name',type:'text'}],rows:[{id:'internal-record-id',values:{name:'设计任务'}}]}};
 const p={id:'projects',database:{columns:[{id:'name',type:'text'},{id:'links',name:'关联任务',type:'relation',targetDatabaseId:'tasks'}],rows:[{id:'project',values:{name:'项目',links:'["internal-record-id"]'}}]}};
 const options={view:'gallery',query:''};let html=renderDatabase(p,[p,target],options);
 assert.match(html,/设计任务/);assert.doesNotMatch(html,/internal-record-id/);
 target.database.rows=[];html=renderDatabase(p,[p,target],options);assert.match(html,/记录已删除或不存在/);
});
test('属性公式实时计算并保留原始值',()=>{
 assert.equal(computeCell(page,page.database.rows[0],'total',[page]).value,100);
 assert.equal(computeCell(page,page.database.rows[0],'status',[page]).value,'达标');
 page.database.rows[0].values.qty='2';assert.equal(computeCell(page,page.database.rows[0],'total',[page]).value,50);page.database.rows[0].values.qty='4';
 assert.equal(page.database.rows[0].values.total,undefined);
});
test('公式拒绝脚本、循环与无效计算，并使用惰性条件',()=>{
 for(const input of ['globalThis.process.exit()', 'constructor("return 1")()', 'prop("单价").constructor', '(()=>1)()'])assert.throws(()=>parseFormula(input));
 const p=structuredClone(page);p.database.columns.push({id:'loop',name:'循环',type:'formula',formula:'prop("循环")'});
 assert.match(computeCell(p,p.database.rows[0],'loop',[p]).error,/循环/);
 p.database.columns[3].formula='1 / 0';assert.match(computeCell(p,p.database.rows[0],'total',[p]).error,/零/);
 p.database.columns[3].formula='if(true, 3, 1 / 0)';assert.equal(computeCell(p,p.database.rows[0],'total',[p]).value,3);
 p.database.columns[3].formula='prop("不存在")';assert.match(computeCell(p,p.database.rows[0],'total',[p]).error,/不存在/);
});
test('文本集合和日期函数',()=>{
 const p=structuredClone(page);const c=p.database.columns[3];c.formula='concat("金额：", format(round(10 / 3, 2)))';assert.equal(computeCell(p,p.database.rows[0],c.id,[p]).value,'金额：3.33');
 c.formula='dateBetween(parseDate("2026-10-13"), parseDate("2026-10-10"), "days")';assert.equal(computeCell(p,p.database.rows[0],c.id,[p]).value,3);
 c.formula='length("中文") + sum(1, 2, 3)';assert.equal(computeCell(p,p.database.rows[0],c.id,[p]).value,8);
});
test('关联汇总可被公式引用，丢失关联明确报告',()=>{
 const tasks={id:'tasks',database:{columns:[{id:'title',name:'任务',type:'text'},{id:'hours',name:'工时',type:'number'}],rows:[{id:'t1',values:{title:'设计',hours:'3'}},{id:'t2',values:{title:'实现',hours:'5'}}]}};
 const projects={id:'projects',database:{columns:[{id:'title',name:'项目',type:'text'},{id:'links',name:'任务',type:'relation',targetDatabaseId:'tasks'},{id:'sum',name:'总工时',type:'rollup',relationId:'links',targetPropertyId:'hours',operation:'sum'},{id:'count',name:'任务数',type:'rollup',relationId:'links',operation:'count'},{id:'budget',name:'预算',type:'formula',formula:'prop("总工时") * 100'}],rows:[{id:'p1',values:{title:'轻笺',links:'["t1","t2"]'}}]}};
 const r=projects.database.rows[0],pages=[projects,tasks];assert.equal(computeCell(projects,r,'sum',pages).value,8);assert.equal(computeCell(projects,r,'count',pages).value,2);assert.equal(computeCell(projects,r,'budget',pages).value,800);
 tasks.database.rows.pop();assert.match(computeCell(projects,r,'sum',pages).error,/删除|不存在/);
});
test('多选值及计算结果表格导出',()=>{
 assert.deepEqual(readSelection('["设计","开发"]'),['设计','开发']);assert.equal(formatValue(['设计','开发']),'设计、开发');
 assert.match(databaseCSV(page,[page]),/笔记本,25,4,100,达标/);
});
test('简易表格保留竖线、空格和多行并能够往返',()=>{
 const rows=[['名称','说明'],['轻笺','甲 | 乙\n第二行'],['','值']];const text=tableSerialize(rows);assert.deepEqual(tableParse(text),rows);
});
test('日历月份正确包含跨月日期',()=>{
 const cells=calendarCells('2026-10');assert.equal(cells.length,42);assert.equal(cells[0],'2026-09-28');assert.ok(cells.includes('2026-10-13'));
});
test('简易表格字面标签和首尾空格不会被转换或丢弃',()=>{
 const rows=[['说明'],[' <br> &lt;  ']];assert.deepEqual(tableParse(tableSerialize(rows)),rows);
});
test('公式对原始长文本与错误日期返回可控错误',()=>{
 const p=structuredClone(page);p.database.columns.push({id:'long',name:'长文本',type:'text'});p.database.rows[0].values.long='x'.repeat(20001);p.database.columns[3].formula='prop("长文本")';assert.match(computeCell(p,p.database.rows[0],'total',[p]).error,/过长/);
 p.database.columns[3].formula='parseDate("2026-02-31")';assert.match(computeCell(p,p.database.rows[0],'total',[p]).error,/日期/);
});
test('简易表格内容行中的分隔符文本不能被当成表头分隔线',()=>{
 const rows=[['内容'],['---'],['最后一行']];assert.deepEqual(tableParse(tableSerialize(rows)),rows);
});
test('汇总溢出明确返回错误',()=>{
 const target={id:'target',database:{columns:[{id:'n',name:'数值',type:'number'}],rows:[{id:'a',values:{n:'1e308'}},{id:'b',values:{n:'1e308'}}]}};
 const p={id:'source',database:{columns:[{id:'rel',name:'关联',type:'relation',targetDatabaseId:'target'},{id:'sum',name:'合计',type:'rollup',relationId:'rel',targetPropertyId:'n',operation:'sum'}],rows:[{id:'r',values:{rel:'["a","b"]'}}]}};
 assert.match(computeCell(p,p.database.rows[0],'sum',[p,target]).error,/范围/);
});
test('再次选择表格类型及文本转表格保留原有内容',async()=>{
 const {tableFromBlock}=await import('../ui/web/database.mjs');const text=tableSerialize([['名称','说明'],['原始内容','备注']]);assert.equal(tableFromBlock({type:'table',text}),text);assert.ok(tableParse(tableFromBlock({type:'paragraph',text:'不能丢失的正文'})).flat().includes('不能丢失的正文'));
});
test('日历将不存在的日期识别为无效，记录不会被藏在某个月份',async()=>{
 const {isCalendarDate}=await import('../ui/web/database.mjs');assert.equal(isCalendarDate('2026-02-31'),false);assert.equal(isCalendarDate('2024-02-29'),true);assert.equal(isCalendarDate('2026-10-13'),true);
});

test('状态与单选使用可配置颜色，状态有圆点且可选择',()=>{
 const p={id:'shows',database:{columns:[{id:'name',name:'名称',type:'text'},{id:'status',name:'是否观看',type:'status',options:['观看中','未观看'],optionColors:{'观看中':'blue','未观看':'red'}},{id:'platform',name:'平台',type:'select',options:['优酷'],optionColors:{'优酷':'purple'}}],rows:[{id:'r',values:{name:'牧神记',status:'观看中',platform:'优酷'}}]}};
 const html=renderDatabase(p,[p],{view:'table',query:''});assert.match(html,/option-blue/);assert.match(html,/status-dot/);assert.match(html,/option-purple/);assert.match(html,/<select[^>]*aria-label="是否观看"/);
 const gallery=renderDatabase(p,[p],{view:'gallery',query:''});assert.match(gallery,/option-blue/);assert.match(gallery,/option-purple/);
});

test('页面菜单包含编辑收藏复制移动与回收站，并按收藏和锁定状态变化',async()=>{
 const m=await import('../ui/web/page-menu.mjs').catch(()=>({}));assert.equal(typeof m.pageMenuItems,'function','页面快捷菜单尚未实现');
 const items=m.pageMenuItems({id:'p',title:'影视剧',favorite:false});assert.deepEqual(items.map(i=>i.action),['pageMenuOpen','pageMenuRename','pageMenuFavorite','pageMenuDuplicate','pageMenuMove','pageMenuTrash']);
 assert.match(items.find(i=>i.action==='pageMenuFavorite').label,/收藏/);assert.equal(m.pageMenuItems({favorite:true,locked:true}).find(i=>i.action==='pageMenuRename').disabled,true);assert.match(m.pageMenuItems({favorite:true}).find(i=>i.action==='pageMenuFavorite').label,/取消/);
});
