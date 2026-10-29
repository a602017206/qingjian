import * as THREE from './vendor/three/three.module.min.js';
import { palettes, roundedShape } from './scene-kit.mjs';
import { createRotation } from './rotation.mjs';

export function createScene(host, { theme = 'paper', paused = false } = {}) {
  let renderer;
  try { renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'low-power' }); }
  catch { host.dataset.ready = 'fallback'; return { available: false }; }

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(35, 1, .1, 30);
  camera.position.set(0, 0, 9.5);
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.7));
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.setClearColor(0x000000, 0);
  host.appendChild(renderer.domElement);
  renderer.domElement.setAttribute('aria-hidden', 'true');
  const materials = [];
  function material(role, extra = {}) {
    const value = new THREE.MeshStandardMaterial({ color: palettes[theme][role], roughness: .68, metalness: .02, ...extra });
    value.userData.role = role; materials.push(value); return value;
  }
  const paper = material('paper'), back = material('back'), edge = material('edge');
  const accent = material('accent'), ink = material('ink'), green = material('green');
  const thinInk = material('ink', { transparent: true, opacity: .55 });
  const sculpture = new THREE.Group();
  scene.add(sculpture);
  const notes = new THREE.Group();
  const turntable = new THREE.Group(), rotation = createRotation();
  sculpture.add(turntable); turntable.add(notes);

  function tile(width, height, radius, depth, mat) {
    const geometry = new THREE.ExtrudeGeometry(roundedShape(THREE, width, height, radius), { depth, bevelEnabled: true, bevelThickness: .012, bevelSize: .012, bevelSegments: 2, steps: 1, curveSegments: 8 });
    return new THREE.Mesh(geometry, mat);
  }
  function bar(parent, width, height, x, y, mat = ink, z = .055) {
    const mesh = tile(width, height, Math.min(height / 2, .05), .012, mat);
    mesh.position.set(x, y, z); parent.add(mesh); return mesh;
  }
  for (let i = 0; i < 4; i++) {
    const sheet = tile(2.35, 3.05, .12, .035, i === 3 ? paper : i === 2 ? edge : back);
    sheet.position.set((3 - i) * .14, (3 - i) * -.12, i * .15 - .3);
    sheet.rotation.z = (3 - i) * -.045;
    notes.add(sheet);
    if (i === 3) {
      bar(sheet, .30, .30, -.73, 1.02, accent);
      bar(sheet, 1.18, .12, -.27, .55, accent);
      bar(sheet, 1.65, .045, -.04, .29, thinInk);
      bar(sheet, 1.38, .045, -.18, .13, thinInk);
      for (let row = 0; row < 3; row++) {
        const y = -.20 - row * .28;
        bar(sheet, .13, .13, -.78, y, row === 2 ? back : green);
        bar(sheet, row === 1 ? 1.1 : .82, .05, row === 1 ? -.02 : -.16, y, ink);
      }
      bar(sheet, .75, .42, -.45, -1.10, back);
      bar(sheet, .75, .42, .45, -1.10, edge);
      bar(sheet, .44, .035, -.45, -1.06, paper, .087);
      bar(sheet, .30, .035, .38, -1.06, paper, .087);
      // 页角的小星标，让真实厚度与高光可见。
      const star = new THREE.Shape();
      for (let p = 0; p < 8; p++) {
        const angle = p / 8 * Math.PI * 2, radius = p % 2 ? .08 : .24;
        const x = Math.cos(angle) * radius, y = Math.sin(angle) * radius;
        if (p === 0) star.moveTo(x, y); else star.lineTo(x, y);
      }
      star.closePath();
      const spark = new THREE.Mesh(new THREE.ExtrudeGeometry(star, { depth: .035, bevelEnabled: false }), green);
      spark.position.set(.78, 1.02, .06); spark.rotation.z = .15; sheet.add(spark);
    }
  }
  notes.rotation.set(.10, -.40, -.17);
  notes.position.set(-.02, .06, .1);

  const smallNotes = [];
  for (const [x, y, z, rotation] of [[-2.05, .65, -.45, .18], [1.85, -.85, .1, -.15]]) {
    const note = new THREE.Group();
    note.add(tile(.78, .95, .075, .03, paper));
    bar(note, .35, .055, -.08, .22, accent);
    bar(note, .48, .025, -.015, .03, ink);
    bar(note, .33, .025, -.09, -.10, thinInk);
    bar(note, .25, .13, -.13, -.29, green);
    note.position.set(x, y, z); note.rotation.set(.1, -.3, rotation);
    note.userData.baseY = y; smallNotes.push(note); sculpture.add(note);
  }

  const orbitGroup = new THREE.Group();
  sculpture.add(orbitGroup);
  for (let i = 0; i < 2; i++) {
    const points = Array.from({ length: 160 }, (_, p) => {
      const a = p / 160 * Math.PI * 2;
      return new THREE.Vector3(Math.cos(a) * (2.8 + i * .18), Math.sin(a) * (1.7 + i * .1), 0);
    });
    const mat = new THREE.LineBasicMaterial({ color: palettes[theme].accent, transparent: true, opacity: i ? .13 : .25 });
    mat.userData.role = 'accent'; materials.push(mat);
    const orbit = new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(points), mat);
    orbit.rotation.set(i ? -.7 : .8, i ? .4 : -.2, i ? .65 : -.6);
    orbitGroup.add(orbit);
  }

  const motes = new THREE.Group();
  sculpture.add(motes);
  const dotGeometry = new THREE.SphereGeometry(.035, 8, 6);
  for (let i = 0; i < 30; i++) {
    const dot = new THREE.Mesh(dotGeometry, i % 3 ? edge : green);
    const angle = i * 2.39996;
    const radius = 2.15 + (i % 5) * .17;
    dot.position.set(Math.cos(angle) * radius, Math.sin(angle) * radius * .78, Math.sin(i * 1.7) * 1.1 - .45);
    dot.scale.setScalar(i % 4 === 0 ? 1.8 : .7); motes.add(dot);
  }
  const diamond = new THREE.Mesh(new THREE.OctahedronGeometry(.14), accent);
  diamond.position.set(1.75, 1.55, .2); sculpture.add(diamond);
  const diamond2 = new THREE.Mesh(new THREE.OctahedronGeometry(.09), green);
  diamond2.position.set(-1.7, -1.4, .25); sculpture.add(diamond2);

  scene.add(new THREE.AmbientLight(0xffffff, 2.4));
  const keyLight = new THREE.DirectionalLight(palettes[theme].light, 3.2);
  keyLight.position.set(-3, 5, 6); scene.add(keyLight);
  const fillLight = new THREE.DirectionalLight(0xa9baff, 1.5);
  fillLight.position.set(4, -2, 2); scene.add(fillLight);

  let visible = true, disposed = false, available = true, lastTime = 0, time = 0, burst = 0;
  const pointer = new THREE.Vector2();
  function render(now = performance.now(), force = false) {
    if (disposed || !available) return;
    if (!force && !paused && lastTime && now - lastTime < 1000 / 40) return;
    const delta = lastTime ? Math.min((now - lastTime) / 1000, .08) : 0;
    lastTime = now;
    if (!paused) {
      rotation.step(delta);
      time += delta;
      burst = Math.max(0, burst - delta * .7);
      sculpture.position.y = Math.sin(time * .6) * .08;
      sculpture.rotation.y += (pointer.x * .16 - sculpture.rotation.y) * .04;
      sculpture.rotation.x += (-pointer.y * .09 - sculpture.rotation.x) * .04;
      notes.rotation.z = -.17 + Math.sin(time * .4) * .035;
      smallNotes.forEach((note, i) => { note.position.y = note.userData.baseY + Math.sin(time * .8 + i * 2) * .13; });
      orbitGroup.rotation.y = Math.sin(time * .12) * .3;
      motes.rotation.z += delta * (.025 + burst * .18);
      diamond.rotation.y += delta * .35; diamond.rotation.z += delta * .18;
      diamond2.rotation.x += delta * .25;
    }
    turntable.rotation.set(rotation.pitch, rotation.yaw, 0);
    renderer.render(scene, camera);
  }
  function syncLoop() {
    if (disposed || !available) return;
    renderer.setAnimationLoop(null); lastTime = 0;
    if (visible && !document.hidden) {
      render();
      if (!paused) renderer.setAnimationLoop(now => render(now));
    }
    host.dataset.animating = String(visible && !document.hidden && !paused);
  }
  function resize() {
    const { width, height } = host.getBoundingClientRect();
    if (!width || !height || disposed || !available) return;
    camera.aspect = width / height;
    camera.position.z = camera.aspect < 1 ? 10.5 : 9.5;
    camera.updateProjectionMatrix(); renderer.setSize(width, height, false); render(performance.now(), true);
  }
  const resizeObserver = new ResizeObserver(resize); resizeObserver.observe(host);
  const intersection = new IntersectionObserver(entries => { visible = entries[0].isIntersecting; syncLoop(); }, { threshold: .01 }); intersection.observe(host);
  function onPointer(event) {
    if (event.pointerId === activePointer) {
      if (pendingTouch) {
        const dx = event.clientX - pendingTouch.x, dy = event.clientY - pendingTouch.y;
        if (Math.abs(dy) > 10 && Math.abs(dy) > Math.abs(dx)) { finishPointer({ pointerId: event.pointerId, type: 'pointercancel' }); return; }
        if (Math.abs(dx) < 7 || Math.abs(dx) < Math.abs(dy) * 1.2) return;
        rotation.begin(pendingTouch.x, pendingTouch.y, pendingTouch.time); pendingTouch = null; host.dataset.dragging = 'true';
      }
      rotation.move(event.clientX, event.clientY); render(performance.now(), true); return;
    }
    if (event.pointerType === 'touch') return;
    const rect = host.getBoundingClientRect();
    pointer.set((event.clientX - rect.left) / rect.width * 2 - 1, (event.clientY - rect.top) / rect.height * 2 - 1);
  }
  function resetPointer() { pointer.set(0, 0); }
  let activePointer = null, pendingTouch = null;
  function onPointerDown(event) {
    if (!event.isPrimary || (event.pointerType === 'mouse' && event.button !== 0)) return;
    activePointer = event.pointerId;
    if (event.pointerType === 'touch') pendingTouch = { x: event.clientX, y: event.clientY, time: performance.now() };
    else rotation.begin(event.clientX, event.clientY);
    pointer.set(0, 0); host.setPointerCapture(event.pointerId); host.dataset.dragging = String(!pendingTouch);
    if (event.pointerType === 'mouse') { event.preventDefault(); host.focus({ preventScroll: true }); }
    if (!paused) burst = 1;
  }
  function finishPointer(event) {
    if (event.pointerId !== activePointer) return;
    rotation.end(!paused && event.type === 'pointerup'); activePointer = null; pendingTouch = null; host.dataset.dragging = 'false';
    if (host.hasPointerCapture(event.pointerId)) host.releasePointerCapture(event.pointerId);
    render(performance.now(), true);
  }
  function resetRotation() {
    rotation.reset(); pointer.set(0, 0); activePointer = null; pendingTouch = null; host.dataset.dragging = 'false';
    render(performance.now(), true);
  }
  function onKey(event) {
    const delta = { ArrowLeft: [-.18, 0], ArrowRight: [.18, 0], ArrowUp: [0, -.12], ArrowDown: [0, .12] }[event.key];
    if (event.altKey || event.ctrlKey || event.metaKey || (!delta && event.key !== 'Home')) return;
    event.preventDefault();
    if (event.key === 'Home') resetRotation();
    else { rotation.nudge(...delta); render(performance.now(), true); }
  }
  host.addEventListener('pointermove', onPointer, { passive: true });
  host.addEventListener('pointerleave', resetPointer);
  host.addEventListener('pointerdown', onPointerDown);
  host.addEventListener('pointerup', finishPointer);
  host.addEventListener('pointercancel', finishPointer);
  host.addEventListener('lostpointercapture', finishPointer);
  host.addEventListener('keydown', onKey);
  document.addEventListener('visibilitychange', syncLoop);
  function onContextLost(event) {
    event.preventDefault(); available = false; renderer.setAnimationLoop(null);
    host.dataset.ready = 'fallback'; host.dataset.animating = 'false';
    renderer.domElement.hidden = true;
    host.dispatchEvent(new CustomEvent('scene-unavailable'));
  }
  renderer.domElement.addEventListener('webglcontextlost', onContextLost);
  function dispose() {
    if (disposed) return;
    disposed = true; renderer.setAnimationLoop(null); resizeObserver.disconnect(); intersection.disconnect();
    const geometries = new Set();
    scene.traverse(object => { if (object.geometry) geometries.add(object.geometry); });
    for (const geometry of geometries) geometry.dispose();
    for (const mat of materials) mat.dispose();
    renderer.dispose();
    document.removeEventListener('visibilitychange', syncLoop);
    host.removeEventListener('pointermove', onPointer); host.removeEventListener('pointerleave', resetPointer); host.removeEventListener('pointerdown', onPointerDown);
    host.removeEventListener('pointerup', finishPointer); host.removeEventListener('pointercancel', finishPointer); host.removeEventListener('lostpointercapture', finishPointer); host.removeEventListener('keydown', onKey);
  }
  window.addEventListener('pagehide', event => { if (event.persisted) { visible = false; syncLoop(); } else dispose(); });
  window.addEventListener('pageshow', event => { if (event.persisted) { visible = true; syncLoop(); } });
  resize(); syncLoop(); host.dataset.ready = 'true';
  return {
    get available() { return available; },
    setPaused(value) { paused = value; if (paused) rotation.end(false); syncLoop(); },
    resetRotation,
    setTheme(value) {
      if (!palettes[value]) return;
      theme = value;
      for (const mat of materials) mat.color.setHex(palettes[theme][mat.userData.role]);
      keyLight.color.setHex(palettes[theme].light); render(performance.now(), true);
    },
    dispose,
  };
}
