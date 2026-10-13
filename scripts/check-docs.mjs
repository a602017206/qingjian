import { readdir, readFile } from 'node:fs/promises';
const files=(await readdir('docs/每日记录')).filter(name=>name.endsWith('.md')), dates=new Set();
for(const file of files){if(!/^\d{4}-\d{2}-\d{2}_\d{4}_.+[\u4e00-\u9fff].*\.md$/.test(file))throw new Error(`记录文件名缺少日期、时间或中文概述：${file}`);const date=file.slice(0,10);if(dates.has(date))throw new Error(`同一天不能创建多份记录：${date}`);dates.add(date);const content=await readFile(`docs/每日记录/${file}`,'utf8');if(!content.startsWith('# ')||!/[\u4e00-\u9fff]/.test(content))throw new Error(`记录缺少中文标题：${file}`);}
console.log('每日记录检查通过');
