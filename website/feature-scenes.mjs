import * as THREE from './vendor/three/three.module.min.js';
import { palettes, roundedShape } from './scene-kit.mjs';

// 四个独立场景共享一个 WebGL 上下文，用裁剪视口放回各自的功能卡片。
export function createFeatureScenes(host, { theme = 'paper', paused = false, text = key => key } = {}) {
  let renderer;
  try { renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, powerPreference: 'low-power' }); }
  catch { host.dataset.effectsReady = 'fallback'; return { available: false }; }
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.4));
  renderer.setClearColor(0x000000, 0); renderer.autoClear = false;
  const canvas = renderer.domElement;
  canvas.className = 'feature-effects-canvas'; canvas.setAttribute('aria-hidden', 'true'); host.appendChild(canvas);
  const materials = [], labels = [], entries = [];
  let available = true, disposed = false, visible = false, time = 0, lastTime = 0;

  function material(role, extra = {}) {
    const mat = new THREE.MeshStandardMaterial({ color: palettes[theme][role], roughness: .55, metalness: .03, ...extra });
    mat.userData.role = role; materials.push(mat); return mat;
  }
  const paper = material('paper'), back = material('back'), edge = material('edge');
  const accent = material('accent'), green = material('green'), ink = material('ink');
  function tile(width, height, mat = paper, depth = .06, radius = .07) {
    return new THREE.Mesh(new THREE.ExtrudeGeometry(roundedShape(THREE, width, height, Math.min(radius, height / 2)), {
      depth, bevelEnabled: true, bevelThickness: .018, bevelSize: .018, bevelSegments: 2, curveSegments: 6, steps: 1,
    }), [mat, mat === paper ? edge : mat]);
  }
  function place(parent, mesh, x, y, z = 0) { mesh.position.set(x, y, z); parent.add(mesh); return mesh; }
  function bar(parent, width, height, x, y, mat = ink, z = .09) { return place(parent, tile(width, height, mat, .01, .025), x, y, z); }
  function label(parent, key, width, x, y, z, role = 'text', literal = '') {
    const drawing = document.createElement('canvas'); drawing.width = literal ? 128 : 1024; drawing.height = 128;
    const texture = new THREE.CanvasTexture(drawing); texture.colorSpace = THREE.SRGBColorSpace;
    const mat = new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthWrite: false, toneMapped: false });
    const mesh = place(parent, new THREE.Mesh(new THREE.PlaneGeometry(1, 1), mat), x, y, z);
    materials.push(mat); labels.push({ drawing, texture, key, role, literal, mesh, maxWidth: width });
    return mesh;
  }
  function updateLabels() {
    for (const { drawing, texture, key, role, literal, mesh, maxWidth } of labels) {
      const ctx = drawing.getContext('2d'); ctx.clearRect(0, 0, drawing.width, drawing.height);
      ctx.font = `550 ${literal ? 92 : 60}px -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif`;
      ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
      ctx.fillStyle = `#${palettes[theme][role].toString(16).padStart(6, '0')}`;
      const value = key ? text(key) : literal;
      ctx.fillText(value, drawing.width / 2, 64, drawing.width - 24);
      const contentWidth = literal ? 128 : Math.min(1024, ctx.measureText(value).width + 48);
      texture.repeat.x = contentWidth / drawing.width; texture.offset.x = (1 - texture.repeat.x) / 2;
      let height = literal ? maxWidth : .36, width = height * contentWidth / 128;
      if (width > maxWidth) { height *= maxWidth / width; width = maxWidth; }
      mesh.scale.set(width, height, 1); texture.needsUpdate = true;
    }
  }
  const sphereGeometry = new THREE.SphereGeometry(.06, 12, 8);
  function dot(parent, mat, x, y, z = 0, scale = 1) {
    const mesh = place(parent, new THREE.Mesh(sphereGeometry, mat), x, y, z); mesh.scale.setScalar(scale); return mesh;
  }
  function path(parent, curve, mat = edge, radius = .017) {
    parent.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 48, radius, 6, false), mat)); return curve;
  }
  function base(kind) {
    const target = host.querySelector(`[data-feature-scene="${kind}"]`), scene = new THREE.Scene();
    scene.add(new THREE.AmbientLight(0xffffff, 1.9));
    const light = new THREE.DirectionalLight(0xfffaf2, 2.3); light.position.set(-3, 5, 8); scene.add(light);
    const fill = new THREE.DirectionalLight(0xc6d7ff, .8); fill.position.set(4, -2, 4); scene.add(fill);
    const group = new THREE.Group(); scene.add(group);
    const camera = new THREE.OrthographicCamera(-3.8, 3.8, 1.5, -1.5, .1, 30); camera.position.z = 9;
    const entry = { target, scene, group, camera, pointer: new THREE.Vector2(), animate: () => {} };
    entries.push(entry); return entry;
  }

  const blocks = base('blocks');
  const writing = new THREE.Group(); blocks.group.add(writing); writing.rotation.set(-.22, -.22, -.035);
  place(writing, tile(4.5, 2.25, back), -.2, -.02, -.10);
  place(writing, tile(4.5, 2.25), -.35, .07, .03);
  label(writing, 'feature.blocks.label', 3.5, -.4, .77, .14, 'accent');
  const rows = [];
  for (let index = 0; index < 2; index++) {
    const row = new THREE.Group(); writing.add(row); row.position.set(-.35, .17 - index * .62, .12); rows.push(row);
    row.add(tile(3.75, .43, index ? back : paper, .035));
    bar(row, .17, .17, -1.48, 0, index ? edge : green, .06);
    label(row, `feature.blocks.todo${index + 1}`, 2.7, .13, 0, .10);
  }
  const slash = place(blocks.group, tile(.8, .8, paper, .13, .12), 2.2, .05, .7);
  slash.rotation.set(-.1, -.25, .15); label(slash, null, .8, 0, 0, .16, 'accent', '/');
  blocks.animate = t => {
    writing.position.y = Math.sin(t * .7) * .05;
    rows.forEach((row, i) => { row.position.z = .15 + Math.sin(t * .9 + i * 1.3) * .08; row.position.x = -.35 + Math.sin(t * .7 + i) * .05; });
    slash.rotation.z = .15 + Math.sin(t * .8) * .12; slash.position.y = Math.sin(t) * .12;
  };

  const database = base('database'); database.group.rotation.set(-.16, -.12, 0);
  for (let column = 0; column < 3; column++) {
    const x = (column - 1) * 2.05;
    place(database.group, tile(1.8, 1.9, column === 1 ? back : paper, .06), x, -.05, -.06);
    bar(database.group, .66, .055, x - .35, .66, column === 1 ? green : accent);
    for (let row = 0; row < 2; row++) {
      const card = place(database.group, tile(1.48, .43, row ? paper : edge, .045), x, -.05 - row * .62, .05);
      bar(card, .82, .027, -.15, .04, row ? ink : paper, .07);
    }
  }
  const movingCard = place(database.group, tile(1.43, .43, paper, .09), -2.05, .12, .25);
  bar(movingCard, .16, .16, -.48, 0, green, .12); bar(movingCard, .65, .035, .12, 0, accent, .12);
  database.animate = t => {
    const p = (Math.sin(t * .55 - Math.PI / 2) + 1) / 2;
    movingCard.position.set(-2.05 + p * 4.1, .1 + Math.sin(p * Math.PI) * .7, .3 + Math.sin(p * Math.PI) * .7);
    movingCard.rotation.z = Math.sin(p * Math.PI * 2) * .10;
  };

  const links = base('links');
  const nodes = [[-2.15, -.22, 'feature.links.node1'], [0, .34, 'feature.links.node2'], [2.15, -.22, 'feature.links.node3']];
  const nodeMeshes = nodes.map(([x, y, key], index) => {
    const node = place(links.group, tile(1.65, .72, index === 1 ? back : paper, .11, .10), x, y, .18);
    label(node, key, 1.35, 0, 0, .15, index === 1 ? 'accent' : 'text'); node.rotation.set(-.15, index === 0 ? .15 : -.15, index === 1 ? -.05 : .05); return node;
  });
  const curves = [
    new THREE.CubicBezierCurve3(new THREE.Vector3(-2.15, -.22, 0), new THREE.Vector3(-1.6, 1.1, -.3), new THREE.Vector3(-.7, 1.1, 0), new THREE.Vector3(0, .34, .1)),
    new THREE.CubicBezierCurve3(new THREE.Vector3(0, .34, .1), new THREE.Vector3(.7, -1.1, .35), new THREE.Vector3(1.6, -1.1, -.1), new THREE.Vector3(2.15, -.22, 0)),
    new THREE.CubicBezierCurve3(new THREE.Vector3(-2.15, -.22, 0), new THREE.Vector3(-1.2, -1.1, 0), new THREE.Vector3(1.2, 1.0, -.3), new THREE.Vector3(2.15, -.22, 0)),
  ];
  const pulses = curves.map((curve, index) => { path(links.group, curve); return dot(links.group, index === 1 ? green : accent, 0, 0, 0, .9); });
  links.animate = t => {
    pulses.forEach((pulse, i) => pulse.position.copy(curves[i].getPointAt((t * .15 + i * .3) % 1)));
    nodeMeshes.forEach((node, i) => { node.position.y = nodes[i][1] + Math.sin(t * .8 + i * 1.8) * .055; });
  };

  const history = base('history'); history.group.rotation.set(-.12, -.10, 0); history.group.position.y = -.35;
  const timeline = new THREE.CatmullRomCurve3(Array.from({ length: 5 }, (_, i) => new THREE.Vector3((i - 2) * 1.3, .25 - (i - 2) ** 2 * .14, 0)));
  path(history.group, timeline, edge, .022);
  for (let i = 0; i < 5; i++) {
    const point = timeline.getPointAt(i / 4); dot(history.group, i === 4 ? green : edge, point.x, point.y, point.z, i === 4 ? 1.8 : 1);
    if (i % 2) continue;
    const page = place(history.group, tile(.72, .90, i === 4 ? back : paper, .07), point.x, point.y + .66, .03);
    page.rotation.z = (i - 2) * -.06;
    bar(page, .38, .045, -.05, .20, i === 4 ? green : accent);
    bar(page, .46, .022, 0, .02); bar(page, .32, .022, -.07, -.13);
  }
  const historyPulse = dot(history.group, green, 0, 0, 0, 1.35);
  history.animate = t => historyPulse.position.copy(timeline.getPointAt((t * .13) % 1));

  function draw(now = performance.now(), force = false) {
    if (disposed || !available) return;
    if (!force && lastTime && now - lastTime < 1000 / 30) return;
    const dt = lastTime ? Math.min((now - lastTime) / 1000, .08) : 0; lastTime = now;
    if (!paused) time += dt;
    const rect = host.getBoundingClientRect();
    renderer.setScissorTest(false); renderer.clear(); renderer.setScissorTest(true);
    for (const entry of entries) {
      const box = entry.target.getBoundingClientRect();
      if (!box.width || !box.height || (!paused && (box.bottom < 70 || box.top > innerHeight))) continue;
      const x = box.left - rect.left, y = rect.bottom - box.bottom, width = box.width, height = box.height;
      entry.camera.top = 3.8 * height / width; entry.camera.bottom = -entry.camera.top; entry.camera.updateProjectionMatrix();
      if (!paused) {
        entry.animate(time);
        entry.group.rotation.y += (entry.pointer.x * .12 - entry.group.rotation.y) * .03;
      }
      renderer.setViewport(x, y, width, height); renderer.setScissor(x, y, width, height); renderer.render(entry.scene, entry.camera);
    }
  }
  function syncLoop() {
    if (disposed || !available) return;
    visible = entries.some(entry => { const r = entry.target.getBoundingClientRect(); return r.bottom > 70 && r.top < innerHeight; });
    renderer.setAnimationLoop(null); lastTime = 0;
    if (visible && !document.hidden) { draw(performance.now(), true); if (!paused) renderer.setAnimationLoop(now => draw(now)); }
    host.dataset.animating = String(visible && !document.hidden && !paused);
  }
  function resize() {
    if (disposed || !available) return;
    const rect = host.getBoundingClientRect();
    if (rect.width && rect.height) { renderer.setSize(rect.width, rect.height, false); draw(performance.now(), true); }
    syncLoop();
  }
  function onMove(event) {
    if (paused) return;
    for (const entry of entries) {
      const rect = entry.target.closest('.feature-card').getBoundingClientRect();
      entry.pointer.x = event.clientX >= rect.left && event.clientX <= rect.right && event.clientY >= rect.top && event.clientY <= rect.bottom ? (event.clientX - rect.left) / rect.width * 2 - 1 : 0;
    }
  }
  function onLeave() { for (const entry of entries) entry.pointer.set(0, 0); }
  function onLost(event) {
    event.preventDefault(); available = false; renderer.setAnimationLoop(null); canvas.hidden = true;
    host.dataset.effectsReady = 'fallback'; host.dataset.animating = 'false'; host.dispatchEvent(new CustomEvent('scene-unavailable'));
  }
  host.addEventListener('pointermove', onMove, { passive: true }); host.addEventListener('pointerleave', onLeave);
  host.addEventListener('transitionend', resize); canvas.addEventListener('webglcontextlost', onLost);
  window.addEventListener('scroll', syncLoop, { passive: true }); document.addEventListener('visibilitychange', syncLoop);
  const observer = new ResizeObserver(resize); observer.observe(host); for (const entry of entries) observer.observe(entry.target);
  function dispose() {
    if (disposed) return; disposed = true; renderer.setAnimationLoop(null); observer.disconnect();
    const geometries = new Set(); for (const entry of entries) entry.scene.traverse(object => { if (object.geometry) geometries.add(object.geometry); });
    for (const geometry of geometries) geometry.dispose(); for (const mat of materials) mat.dispose(); for (const label of labels) label.texture.dispose();
    renderer.dispose(); window.removeEventListener('scroll', syncLoop); document.removeEventListener('visibilitychange', syncLoop);
    host.removeEventListener('pointermove', onMove); host.removeEventListener('pointerleave', onLeave); host.removeEventListener('transitionend', resize);
  }
  window.addEventListener('pagehide', event => { if (event.persisted) { renderer.setAnimationLoop(null); } else dispose(); });
  window.addEventListener('pageshow', event => { if (event.persisted) resize(); });
  for (const entry of entries) entry.animate(0);
  updateLabels(); host.dataset.effectsReady = 'true'; resize();
  return {
    get available() { return available; },
    setPaused(value) { paused = value; syncLoop(); },
    setTheme(value) { if (!palettes[value]) return; theme = value; for (const mat of materials) if (mat.userData.role) mat.color.setHex(palettes[theme][mat.userData.role]); updateLabels(); draw(performance.now(), true); },
    setLanguage() { updateLabels(); draw(performance.now(), true); },
    dispose,
  };
}
