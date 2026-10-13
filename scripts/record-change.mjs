import { mkdir,readdir,appendFile,rmdir } from 'node:fs/promises';
import { resolve,join } from 'node:path';
import { fileURLToPath } from 'node:url';

export async function appendRecord(root,topic,content,instant=new Date()){
 if(!/[\u4e00-\u9fff]/.test(topic)||!content.trim())throw new Error('请提供中文主题和记录内容');
 const parts=Object.fromEntries(new Intl.DateTimeFormat('zh-CN',{timeZone:'Asia/Shanghai',year:'numeric',month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',hourCycle:'h23'}).formatToParts(instant).map(part=>[part.type,part.value]));
 const date=`${parts.year}-${parts.month}-${parts.day}`,time=`${parts.hour}:${parts.minute}`;
 await mkdir(root,{recursive:true});const lock=join(root,`.record-${date}.lock`);
 try{await mkdir(lock);}catch{throw new Error('当天记录正在被其他进程修改，请稍后重试');}
 try{
  const files=(await readdir(root)).filter(file=>file.startsWith(date+'_')&&file.endsWith('.md'));
  if(files.length>1)throw new Error('当天已有多份记录，请先整理为一个文件');
  const safeTopic=topic.replace(/[\\/:*?"<>|\n\r]/g,'').slice(0,50);
  const file=join(root,files[0]||`${date}_${parts.hour}${parts.minute}_${safeTopic}.md`);
  const prefix=files.length?'':`# ${parts.year}年${parts.month}月${parts.day}日：${topic}\n`;
  await appendFile(file,`${prefix}\n## ${time} ${topic}\n\n${content.trim()}\n`,'utf8');return file;
 }finally{await rmdir(lock);}
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 const [topic,...content]=process.argv.slice(2);
 if(!topic||!content.length){console.error('用法：提供中文主题和记录内容');process.exitCode=1;}
 else console.log(await appendRecord(resolve('docs/每日记录'),topic,content.join('\n')));
}
