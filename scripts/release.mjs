import { readFile,writeFile,readdir,stat,mkdir,copyFile,appendFile } from 'node:fs/promises';
import { resolve,join,dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
export const releaseAssets=['qingjian-windows-x64.exe','qingjian-macos-arm64.zip','qingjian-macos-x64.zip','qingjian-linux-x64.deb','qingjian-linux-x64.AppImage','qingjian-android-arm64.apk','qingjian-ios-simulator-arm64.zip','qingjian-ios-unsigned-arm64.ipa'];
export async function checkVersion(root,tag){
 const json=async path=>JSON.parse(await readFile(join(root,path),'utf8'));
 const p=await json('package.json'),lock=await json('package-lock.json'),config=await json('src-tauri/tauri.conf.json'),version=p.version;
 if(!/^\d+\.\d+\.\d+$/.test(version)||lock.version!==version||lock.packages[''].version!==version||config.version!==version)throw new Error('前端与原生配置版本不一致');
 for(const path of ['src-tauri/Cargo.toml','crates/notebook/Cargo.toml']){const content=await readFile(join(root,path),'utf8');if(content.match(/^version\s*=\s*"([^"]+)"/m)?.[1]!==version)throw new Error('核心与应用版本不一致');}
 const cargo=(await readFile(join(root,'Cargo.lock'),'utf8')).replace(/\r\n/g,'\n');for(const name of ['qingjian','notebook-core'])if(!cargo.includes(`name = "${name}"\nversion = "${version}"`))throw new Error('原生锁文件版本不一致');
 if(tag&&tag!=='v'+version)throw new Error('标签与应用版本不一致');return version;
}
export async function verifyAssets(root){
 const actual=await readdir(root);if(actual.some(name=>!releaseAssets.includes(name)&&name!=='SHA256SUMS.txt'))throw new Error('目录包含非发布文件，停止上传');
 const entries=[];for(const name of releaseAssets){let info;try{info=await stat(join(root,name));}catch{throw new Error('缺少发布产物：'+name);}if(!info.isFile()||info.size===0)throw new Error('发布产物为空：'+name);entries.push({name,size:info.size,sha256:createHash('sha256').update(await readFile(join(root,name))).digest('hex')});}
 await writeFile(join(root,'SHA256SUMS.txt'),entries.map(e=>`${e.sha256}  ${e.name}`).join('\n')+'\n');return entries;
}
async function walk(root){const files=[];for(const entry of await readdir(root,{withFileTypes:true})){if(entry.name.startsWith('.'))continue;const path=join(root,entry.name);if(entry.isDirectory())files.push(...await walk(path));else if(entry.isFile())files.push(path);}return files;}
export async function collectAssets(platform,source,output){
 const suffixes={windows:['.exe'],linux:['.deb','.AppImage'],'macos-arm64':['.zip'],'macos-x64':['.zip']};if(!suffixes[platform])throw new Error('产物平台无效');
 const files=(await stat(source)).isFile()?[source]:await walk(source);await mkdir(output,{recursive:true});
 for(const suffix of suffixes[platform]){const candidates=files.filter(path=>path.endsWith(suffix));if(candidates.length!==1)throw new Error('应当只有一份对应产物：'+platform+suffix);const name=platform==='windows'?'qingjian-windows-x64.exe':platform==='linux'?'qingjian-linux-x64'+suffix:'qingjian-'+platform+'.zip';await copyFile(candidates[0],join(output,name));}
}
async function main(){const [action,...args]=process.argv.slice(2);if(action==='check-version'){const version=await checkVersion(process.cwd(),args[0]||process.env.RELEASE_TAG);if(process.env.GITHUB_OUTPUT)await appendFile(process.env.GITHUB_OUTPUT,`version=${version}\ntag=v${version}\n`);console.log('版本校验通过：'+version);}else if(action==='collect'){await collectAssets(args[0],resolve(args[1]),resolve(args[2]||'artifacts/release'));console.log('发布产物整理完成');}else if(action==='verify-assets'){const entries=await verifyAssets(resolve(args[0]||'release-assets'));console.log('所有平台发布产物及摘要已核对：'+entries.length);}else throw new Error('请指定版本校验、产物整理或完整产物校验命令');}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(error=>{console.error(error.message);process.exitCode=1;});
