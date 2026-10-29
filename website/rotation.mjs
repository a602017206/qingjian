// 输入和惯性独立于渲染器，暂停时仍可由用户主动调整朝向。
export function createRotation() {
  let yaw = 0, pitch = 0, dragging = false, lastX = 0, lastY = 0, lastTime = 0, vx = 0, vy = 0;
  const clampPitch = value => Math.max(-1.1, Math.min(1.1, value));
  return {
    get yaw() { return yaw; }, get pitch() { return pitch; }, get dragging() { return dragging; },
    begin(x, y, time = performance.now()) { dragging = true; lastX = x; lastY = y; lastTime = time; vx = vy = 0; },
    move(x, y, time = performance.now()) {
      if (!dragging) return;
      const dx = (x - lastX) * .008, dy = (y - lastY) * .006;
      const dt = Math.max(.016, (time - lastTime) / 1000);
      yaw += dx; pitch = clampPitch(pitch + dy);
      vx = Math.max(-5, Math.min(5, dx / dt)); vy = Math.max(-3, Math.min(3, dy / dt));
      lastX = x; lastY = y; lastTime = time;
    },
    end(inertia = true) { dragging = false; if (!inertia) vx = vy = 0; },
    step(seconds) {
      if (dragging) return;
      const dt = Math.max(0, Math.min(seconds, .05));
      yaw += vx * dt; pitch = clampPitch(pitch + vy * dt);
      const decay = Math.exp(-7 * dt); vx *= decay; vy *= decay;
      if (Math.abs(vx) < .015) vx = 0;
      if (Math.abs(vy) < .015) vy = 0;
    },
    nudge(x, y) { yaw += x; pitch = clampPitch(pitch + y); vx = vy = 0; },
    reset() { yaw = pitch = vx = vy = 0; dragging = false; },
  };
}
