import test from 'node:test';
import assert from 'node:assert/strict';
import {api} from '../ui/web/api.mjs';
test('原生字符串错误转为真实错误对象，保留同步失败原因',async()=>{
 globalThis.isTauri=true;globalThis.window={__TAURI_INTERNALS__:{invoke:async()=>{throw '连接远程失败：帐号或令牌不正确';}}};
 try{await assert.rejects(api('sync',{}),error=>error instanceof Error&&error.message==='连接远程失败：帐号或令牌不正确');}finally{delete globalThis.isTauri;delete globalThis.window;}
});
test('原生空错误有中文兜底，并且认证信息不进入错误消息',async()=>{
 globalThis.isTauri=true;globalThis.window={__TAURI_INTERNALS__:{invoke:async()=>{throw '认证失败 https://user:private-password@gitee.com/u/notes.git?access_token=query-secret anonymous-test-token';}}};
 try{await assert.rejects(api('createWorkspace',{token:'anonymous-test-token'}),error=>error instanceof Error&&!/anonymous-test-token|private-password|query-secret/.test(error.message));
 window.__TAURI_INTERNALS__.invoke=async()=>{throw undefined;};await assert.rejects(api('sync'),error=>error instanceof Error&&/操作未完成/.test(error.message)&&!error.message.includes('undefined'));
 }finally{delete globalThis.isTauri;delete globalThis.window;}
});
