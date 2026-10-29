import test from 'node:test';
import assert from 'node:assert/strict';

const { createRotation } = await import('../website/rotation.mjs').catch(() => ({}));
test('纸页旋转模型提供可交互控制', () => assert.equal(typeof createRotation, 'function', '尚未提供纸页拖动旋转'));
test('拖动改变朝向，俯仰有限制，复位恢复起点', () => {
  assert.equal(typeof createRotation, 'function');
  const rotation = createRotation();
  rotation.begin(100, 100, 0); rotation.move(260, 160, 120);
  assert.ok(rotation.yaw > .5); assert.ok(rotation.pitch > .1);
  rotation.move(260, 100000, 240); assert.ok(rotation.pitch <= 1.1);
  rotation.reset(); assert.equal(rotation.yaw, 0); assert.equal(rotation.pitch, 0); assert.equal(rotation.dragging, false);
});
test('松手惯性延续后衰减，暂停不自动旋转', () => {
  assert.equal(typeof createRotation, 'function');
  const rotation = createRotation();
  rotation.begin(0, 0, 0); rotation.move(80, 10, 100); rotation.end();
  const start = rotation.yaw; rotation.step(.016); assert.ok(rotation.yaw > start);
  for (let i = 0; i < 300; i++) rotation.step(.016);
  const settled = rotation.yaw; rotation.step(.016); assert.equal(rotation.yaw, settled);
  rotation.begin(0, 0, 6000); rotation.move(50, 30, 6100); rotation.end(false);
  const paused = rotation.yaw; rotation.step(.04); assert.equal(rotation.yaw, paused);
});
test('键盘旋转与复位不残留旧惯性', () => {
  assert.equal(typeof createRotation, 'function');
  const rotation = createRotation();
  rotation.nudge(.2, -.1); assert.equal(rotation.yaw, .2); assert.equal(rotation.pitch, -.1);
  rotation.step(.05); assert.equal(rotation.yaw, .2);
  rotation.reset(); rotation.step(.05); assert.equal(rotation.yaw, 0);
});
