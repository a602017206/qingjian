import test from 'node:test';
import assert from 'node:assert/strict';
const mobile=await import('../ui/web/mobile-ui.mjs').catch(()=>({}));
test('底部面板避开软键盘并支持可见视口偏移',()=>{
 assert.equal(typeof mobile.viewportGeometry,'function');
 assert.deepEqual(mobile.viewportGeometry(844,{height:420,offsetTop:80}),{top:80,height:420,bottom:500,inset:344});
 assert.deepEqual(mobile.viewportGeometry(844),{top:0,height:844,bottom:844,inset:0});
 assert.deepEqual(mobile.viewportGeometry(390,{height:240,offsetTop:0}),{top:0,height:240,bottom:240,inset:150});
});
