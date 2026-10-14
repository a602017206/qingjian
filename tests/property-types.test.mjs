import test from 'node:test';
import assert from 'node:assert/strict';
import { propertyTypes, readFiles, readLocation, readPersons } from '../ui/web/property-types.mjs';
import { computeCell, formatValue, databaseCSV } from '../ui/web/database.mjs';
import { renderCell, renderColumnForm } from '../ui/web/database-ui.mjs';
const types=['text','number','select','multiselect','status','date','person','files','checkbox','url','email','phone','formula','relation','rollup','createdTime','createdBy','editedTime','editedBy','button','location','uniqueId'];
const make=(type,raw='')=>{const page={id:'p',database:{columns:[{id:'name',name:'名称',type:'text'},{id:'field',name:'属性',type}],rows:[]}};const row={id:'r',values:{name:'记录',field:raw},createdAt:'2026-10-13T10:00:00Z',updatedAt:'2026-10-14T10:00:00Z',createdBy:'作者',updatedBy:'编辑者',sequence:7};page.database.rows=[row];return [page,row,page.database.columns[1]];};
test('类型目录按截图排列全部二十二种本地属性',()=>assert.deepEqual(propertyTypes,types));
test('人员、文件和位置保持结构和遗留原文',()=>{
 assert.deepEqual(readPersons('["甲","乙","甲"]'),['甲','乙']);assert.deepEqual(readPersons('旧人员'),['旧人员']);
 assert.deepEqual(readFiles(''),[]);assert.deepEqual(readFiles('[{"name":"照片","asset":"asset:a"},{"name":"站点","url":"https://example.com"}]'),[{name:'照片',asset:'asset:a'},{name:'站点',url:'https://example.com'}]);
 assert.deepEqual(readFiles('旧附件记录'),[{name:'旧附件记录'}]);assert.deepEqual(readLocation('地点'),{address:'地点'});assert.deepEqual(readLocation('{"address":"北京","latitude":39.9,"longitude":116.4}'),{address:'北京',latitude:39.9,longitude:116.4});
 assert.throws(()=>readLocation('{"address":"原始","latitude":200,"longitude":3}'),/位置|纬度/);assert.throws(()=>readFiles('[{"name":3}]'),/文件/);
});
test('系统属性只读计算，元数据优先并兼容导出原值，关联子页编辑时间取较新',()=>{
 for(const [type,expected] of [['createdTime','2026-10-13T10:00:00Z'],['createdBy','作者'],['editedTime','2026-10-14T10:00:00Z'],['editedBy','编辑者'],['uniqueId','7']]){const [p,r,c]=make(type);assert.equal(String(computeCell(p,r,c.id,[p]).value),expected);assert.match(renderCell(p,r,c,[p]),/computed-cell/);assert.doesNotMatch(renderCell(p,r,c,[p]),/<input/);r.values.field='导出原值';assert.equal(String(computeCell(p,r,c.id,[p]).value),expected);for(const key of ['createdAt','createdBy','updatedAt','updatedBy','sequence'])delete r[key];assert.equal(computeCell(p,r,c.id,[p]).value,'导出原值');}
 const [p,r,c]=make('editedTime');r.pageId='child';const child={id:'child',updatedAt:'2026-10-14T11:00:00Z',updatedBy:'子页作者'};assert.equal(computeCell(p,r,c.id,[p,child]).value,child.updatedAt);c.type='editedBy';assert.equal(computeCell(p,r,c.id,[p,child]).value,'子页作者');c.type='uniqueId';c.uniqueIdPrefix='任务';assert.equal(computeCell(p,r,c.id,[p,child]).value,'任务-7');
});
test('复杂属性可编辑且锁定，邮件电话不会自动外发',()=>{
 for(const [type,action] of [['person','selectValues'],['files','editFiles'],['location','editLocation'],['button','runPropertyButton']]){const [p,r,c]=make(type);const html=renderCell(p,r,c,[p],true);assert.match(html,new RegExp(`data-action="${action}"`));assert.match(html,/disabled/);}
 for(const [type,input] of [['email','email'],['phone','tel']]){const [p,r,c]=make(type);assert.match(renderCell(p,r,c,[p]),new RegExp(`type="${input}"`));}
});
test('文件与位置导出可读而不是对象字符串，错误原值可在控件重新编辑',()=>{
 const [p,r,c]=make('files','[{"name":"海报","asset":"asset:abc"}]');assert.equal(formatValue(computeCell(p,r,c.id,[p]).value),'海报 (asset:abc)');assert.match(databaseCSV(p,[p]),/海报 \(asset:abc\)/);
 c.type='location';r.values.field='{"address":"北京","latitude":39.9,"longitude":116.4}';assert.equal(formatValue(computeCell(p,r,c.id,[p]).value),'北京 (39.9, 116.4)');
 r.values.field='{"latitude":300}';const html=renderCell(p,r,c,[p]);assert.match(html,/editLocation/);assert.match(html,/calculation-error/);assert.match(html,/300/);
});
test('属性设置包含全部类型与人员按钮编号配置',()=>{
 const [p,r,c]=make('button');const html=renderColumnForm(p,[p],c);for(const type of types)assert.match(html,new RegExp(`value="${type}"`));for(const name of ['options','buttonLabel','buttonPropertyId','buttonValue','uniqueIdPrefix'])assert.match(html,new RegExp(`name="${name}"`));
});

test('未知类型保留原始类型与内容，空文件条目不能被转换成无意义对象',()=>{
 const [p,r,c]=make('futureType','旧内容');assert.equal(computeCell(p,r,c.id,[p]).value,'旧内容');assert.match(renderColumnForm(p,[p],c),/value="futureType" selected/);assert.throws(()=>readFiles('[""]'),/文件/);
});
