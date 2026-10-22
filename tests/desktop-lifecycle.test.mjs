import test from 'node:test';import assert from 'node:assert/strict';import {readFile} from 'node:fs/promises';
test('桌面系统退出进入保存确认路径，不能绕过常驻菜单',async()=>{const source=await readFile('src-tauri/src/lib.rs','utf8');assert.match(source,/RunEvent::ExitRequested/);assert.match(source,/desktop::exit_requested/);});
