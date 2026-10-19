import { readFile,writeFile,readdir } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { resolve,join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function addIosSystemLibrary(source){
 if(/^\s*- sdk: libiconv\.tbd\s*$/m.test(source))return source;
 const anchors=[...source.matchAll(/^([ \t]*)- sdk: Security\.framework(\r?)$/gm)];
 if(anchors.length!==1)throw new Error('生成工程的系统库位置不唯一，停止配置');
 return source.replace(/^([ \t]*)- sdk: Security\.framework(\r?)$/m,(_,indent,cr)=>`${indent}- sdk: libiconv.tbd${cr}\n${indent}- sdk: Security.framework${cr}`);
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 if(process.env.GITHUB_ACTIONS!=='true')throw new Error('移动原生工程配置仅在云端执行');
 const root=resolve('src-tauri/gen/apple'),spec=join(root,'project.yml');
 await writeFile(spec,addIosSystemLibrary(await readFile(spec,'utf8')));
 const result=spawnSync('xcodegen',['generate','--spec',spec],{cwd:root,stdio:'inherit'});
 if(result.error)throw result.error;
 if(result.status!==0)throw new Error('苹果移动工程生成失败');
 const projects=(await readdir(root)).filter(name=>name.endsWith('.xcodeproj'));
 if(projects.length!==1||!(await readFile(join(root,projects[0],'project.pbxproj'),'utf8')).includes('libiconv.tbd'))throw new Error('苹果移动工程缺少系统字符转换链接');
 console.log('苹果移动工程已明确链接系统字符转换库');
}
