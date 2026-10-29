// 清风叶片只在鼠标移动时生成；独立于 WebGL，保留原生光标与点击行为。
const colors = {
  paper: ['#69a98e', '#92bea3', '#a9bdba', '#b5a4d0'],
  midnight: ['#86cfb2', '#b5d39b', '#b5a6df', '#7abbb8'],
  grove: ['#4e9773', '#80ad82', '#a0b36b', '#bbc58d'],
};

export function createCursorTrail({ theme = 'paper', paused = false } = {}) {
  const canvas = document.createElement('canvas');
  canvas.className = 'cursor-trail'; canvas.hidden = true; canvas.setAttribute('aria-hidden', 'true');
  const context = canvas.getContext('2d');
  if (!context) return { setPaused() {}, setTheme() {} };
  document.body.appendChild(canvas);
  const finePointer = matchMedia('(any-pointer: fine)');
  let leaves = [], last = null, carried = 0, frame = 0, previous = 0, serial = 0, disposed = false;
  let width = innerWidth, height = innerHeight;

  function clear() {
    cancelAnimationFrame(frame); frame = previous = 0; leaves = []; last = null; carried = 0;
    context.clearRect(0, 0, width, height); canvas.hidden = true;
  }
  function resize() {
    clear(); width = innerWidth; height = innerHeight;
    const ratio = Math.min(devicePixelRatio, 1.5);
    canvas.width = Math.round(width * ratio); canvas.height = Math.round(height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
  }
  function drawLeaf(leaf, age) {
    const palette = colors[theme] || colors.paper;
    const alpha = Math.min(1, age / .07) * (1 - age / leaf.life) ** 1.4 * .75;
    const size = leaf.size, angle = leaf.angle + leaf.spin * age + Math.sin(age * 3 + leaf.phase) * .3;
    context.save(); context.translate(leaf.x, leaf.y); context.rotate(angle);
    context.scale(.55 + Math.abs(Math.cos(age * 3.4 + leaf.phase)) * .45, 1);
    context.globalAlpha = alpha;
    if (leaf.wisp) {
      context.globalAlpha = alpha * .24; context.strokeStyle = palette[leaf.color]; context.lineWidth = .65;
      context.beginPath(); context.moveTo(-size * 1.1, -size * .1);
      context.quadraticCurveTo(-size * 2.2, -size * .65, -size * 3.4, -size * .2); context.stroke();
      context.globalAlpha = alpha;
    }
    const gradient = context.createLinearGradient(-size, -size * .4, size, size * .3);
    gradient.addColorStop(0, palette[leaf.color]); gradient.addColorStop(1, palette[(leaf.color + 1) % palette.length]);
    context.fillStyle = gradient;
    context.beginPath(); context.moveTo(-size, 0);
    context.bezierCurveTo(-size * .25, -size * .95, size * .55, -size * .55, size, 0);
    context.bezierCurveTo(size * .3, size * .62, -size * .6, size * .65, -size, 0); context.fill();
    context.globalAlpha = alpha * .58; context.strokeStyle = '#f1fff0'; context.lineWidth = .65;
    context.beginPath(); context.moveTo(-size * .85, 0); context.quadraticCurveTo(0, -.1 * size, size * .82, 0);
    context.moveTo(-size * .25, 0); context.lineTo(size * .03, -size * .33);
    context.moveTo(size * .18, 0); context.lineTo(size * .42, size * .22); context.stroke();
    context.restore();
  }
  function animate(now) {
    frame = 0;
    if (disposed || paused || document.hidden) { clear(); return; }
    if (previous && now - previous < 1000 / 45) { frame = requestAnimationFrame(animate); return; }
    const dt = previous ? Math.min((now - previous) / 1000, .05) : .016; previous = now;
    context.clearRect(0, 0, width, height);
    leaves = leaves.filter(leaf => now - leaf.born < leaf.life * 1000);
    for (const leaf of leaves) {
      const age = (now - leaf.born) / 1000;
      leaf.vx *= Math.exp(-dt * 1.4); leaf.vy += dt * 13;
      leaf.x += (leaf.vx + 9 + Math.sin(age * 2 + leaf.phase) * 6) * dt;
      leaf.y += leaf.vy * dt;
      drawLeaf(leaf, age);
    }
    if (leaves.length) frame = requestAnimationFrame(animate);
    else clear();
  }
  function onMove(event) {
    if (event.pointerType !== 'mouse' || event.buttons || paused || document.hidden || !finePointer.matches || document.querySelector('dialog[open]')) { last = null; carried = 0; return; }
    const now = performance.now(), point = { x: event.clientX, y: event.clientY, time: now };
    if (!last) { last = point; return; }
    const dx = point.x - last.x, dy = point.y - last.y, distance = Math.hypot(dx, dy);
    if (distance > 220) { last = point; carried = 0; return; }
    carried += distance;
    const count = Math.min(3, Math.floor(carried / 20));
    if (count) {
      carried %= 20;
      const dt = Math.max(.016, (now - last.time) / 1000);
      for (let i = 0; i < count; i++) {
        const p = (i + .5) / count;
        leaves.push({ x: last.x + dx * p, y: last.y + dy * p, born: now,
          vx: Math.max(-30, Math.min(30, dx / dt * .035)) + (Math.random() - .5) * 14,
          vy: Math.max(-20, Math.min(20, dy / dt * .025)) - 9 + Math.random() * 9,
          size: 5 + Math.random() * 5, angle: Math.atan2(dy, dx) + (Math.random() - .5),
          spin: (Math.random() - .5) * 2.4, phase: Math.random() * Math.PI * 2,
          life: 1.25 + Math.random() * .5, color: serial++ % 4, wisp: serial % 4 === 0 });
      }
      if (leaves.length > 36) leaves.splice(0, leaves.length - 36);
      canvas.hidden = false;
      if (!frame) frame = requestAnimationFrame(animate);
    }
    last = point;
  }
  function onExit(event) { if (!event.relatedTarget) { last = null; carried = 0; } }
  function onVisibility() { if (document.hidden) clear(); }
  window.addEventListener('pointermove', onMove, { passive: true });
  window.addEventListener('pointerout', onExit, { passive: true });
  window.addEventListener('pointerdown', clear, { passive: true });
  window.addEventListener('blur', clear); window.addEventListener('resize', resize);
  document.addEventListener('visibilitychange', onVisibility); finePointer.addEventListener('change', clear);
  function dispose() {
    disposed = true; clear(); canvas.remove();
    window.removeEventListener('pointermove', onMove); window.removeEventListener('pointerout', onExit);
    window.removeEventListener('pointerdown', clear); window.removeEventListener('blur', clear); window.removeEventListener('resize', resize);
    document.removeEventListener('visibilitychange', onVisibility); finePointer.removeEventListener('change', clear);
  }
  window.addEventListener('pagehide', event => { if (event.persisted) clear(); else dispose(); });
  window.addEventListener('pageshow', event => { if (event.persisted) resize(); });
  resize();
  return { setPaused(value) { paused = value; if (value) clear(); }, setTheme(value) { if (colors[value]) theme = value; }, dispose };
}
