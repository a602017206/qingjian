import test from 'node:test';import assert from 'node:assert/strict';
import {compareVersions,evaluateRelease,fetchLatestRelease,UpdateChecker} from '../ui/web/updates.mjs';
const release=(version='0.0.3')=>({tag_name:'v'+version,html_url:'https://github.com/a602017206/qingjian/releases/tag/v'+version,draft:false,prerelease:false,body:'新版说明',assets:['qingjian-android-arm64.apk','qingjian-windows-x64.exe','qingjian-macos-arm64.zip','qingjian-macos-x64.zip','qingjian-linux-x64.AppImage','qingjian-linux-x64.deb','qingjian-ios-simulator-arm64.zip','qingjian-ios-unsigned-arm64.ipa'].map(name=>({name,size:100,browser_download_url:`https://github.com/a602017206/qingjian/releases/download/v${version}/${name}`}))});
test('版本按数字比较并拒绝格式错误，不降级',()=>{assert.ok(compareVersions('0.0.10','0.0.2')>0);assert.equal(compareVersions('1.2.3+build','v1.2.3'),0);assert.throws(()=>compareVersions('1.02.3','1.2.3'));assert.equal(evaluateRelease({version:'0.0.4',os:'android',arch:'aarch64'},release()).newer,false);});
test('全部原生系统架构准确匹配，未知架构与浏览器不误下载',()=>{
 for(const [os,arch,file,extra]of [['android','aarch64','android-arm64.apk'],['windows','x86_64','windows-x64.exe'],['macos','aarch64','macos-arm64.zip'],['macos','x86_64','macos-x64.zip'],['linux','x86_64','linux-x64.AppImage'],['ios','aarch64','ios-unsigned-arm64.ipa'],['ios','aarch64','ios-simulator-arm64.zip',{simulator:true}]]){const r=evaluateRelease({version:'0.0.2',os,arch,...extra},release());assert.equal(r.downloads[0].name,'qingjian-'+file);}
 assert.equal(evaluateRelease({version:'0.0.2',os:'windows',arch:'aarch64'},release()).downloads.length,0);assert.equal(evaluateRelease({version:'0.0.2',os:'web',arch:'unknown'},release()).downloads.length,0);
});
test('草稿、预览、缺少文件及错误下载地址不会成为可下载更新',()=>{
 for(const key of ['draft','prerelease'])assert.throws(()=>evaluateRelease({version:'0.0.2'}, {...release(),[key]:true}));
 const info={version:'0.0.2',os:'android',arch:'aarch64'},r=release();r.assets=[];assert.equal(evaluateRelease(info,r).downloads.length,0);
 for(const url of ['https://evil.example/app.apk','http://github.com/a602017206/qingjian/releases/download/v0.0.3/qingjian-android-arm64.apk','https://github.com/a602017206/qingjian/releases/download/v0.0.2/qingjian-android-arm64.apk']){const r=release();r.assets[0].browser_download_url=url;assert.throws(()=>evaluateRelease(info,r));}
});
test('网络失败、限流和超大响应明确失败，不冒充最新版',async()=>{
 for(const status of [403,404,429,500])await assert.rejects(fetchLatestRelease(async()=>({ok:false,status})),/update/);
 await assert.rejects(fetchLatestRelease(async()=>({ok:true,headers:{get:()=>null},text:async()=>'x'.repeat(1048577)})),/update/);
 await assert.rejects(fetchLatestRelease(async()=>{throw Error('offline');}),/update/);
});
test('并发检查复用请求，检查失败不显示当前最新版',async()=>{
 let calls=0,complete;const checker=new UpdateChecker({info:async()=>({version:'0.0.2',os:'android',arch:'aarch64'}),fetchRelease:()=>{calls++;return new Promise(resolve=>complete=resolve);}});const a=checker.check(),b=checker.check();await new Promise(resolve=>setTimeout(resolve,0));assert.equal(calls,1);complete(release());await Promise.all([a,b]);assert.equal(checker.state.status,'available');
 const failed=new UpdateChecker({info:async()=>({version:'0.0.2'}),fetchRelease:async()=>{throw Error('offline');}});await failed.check();assert.equal(failed.state.status,'error');assert.equal(failed.state.newer,undefined);
});
