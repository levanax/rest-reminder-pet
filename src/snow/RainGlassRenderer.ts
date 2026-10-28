/**
 * Cold-glass condensation fog (车里冷气、外面湿热 → 玻璃内侧慢慢起雾).
 * No falling rain streaks — mottled milky veil + still dew beads.
 */

type Patch = {
  x: number;
  y: number;
  r: number;
  /** 0–1 local phase offset so fog appears unevenly */
  delay: number;
  strength: number;
};

type Dew = {
  x: number;
  y: number;
  r: number;
  born: number;
};

let canvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let running = false;
let paused = false;
let raf = 0;
let w = 0;
let h = 0;
let startedAt = 0;
let patches: Patch[] = [];
let dews: Dew[] = [];
/** Seconds until fog is nearly opaque. */
const FOG_SECONDS = 70;

function resize() {
  if (!canvas || !ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function seedPatches() {
  const area = Math.max(1, w * h);
  const n = Math.min(90, Math.max(28, Math.floor(area / 22000)));
  patches = [];
  for (let i = 0; i < n; i++) {
    patches.push({
      x: Math.random() * w,
      y: Math.random() * h,
      r: Math.min(w, h) * (0.12 + Math.random() * 0.28),
      delay: Math.random() * 0.55,
      strength: 0.35 + Math.random() * 0.65,
    });
  }
  // Slightly denser near edges (common on car glass)
  for (let i = 0; i < Math.floor(n * 0.35); i++) {
    const edge = Math.floor(Math.random() * 4);
    const x =
      edge === 0
        ? Math.random() * w * 0.18
        : edge === 1
          ? w * (0.82 + Math.random() * 0.18)
          : Math.random() * w;
    const y =
      edge === 2
        ? Math.random() * h * 0.18
        : edge === 3
          ? h * (0.82 + Math.random() * 0.18)
          : Math.random() * h;
    patches.push({
      x,
      y,
      r: Math.min(w, h) * (0.1 + Math.random() * 0.22),
      delay: Math.random() * 0.35,
      strength: 0.5 + Math.random() * 0.5,
    });
  }
}

/** Global condensation progress 0→1 with slow start, then thickens. */
function progress(now: number): number {
  const t = Math.max(0, (now - startedAt) / 1000);
  const u = Math.min(1, t / FOG_SECONDS);
  return Math.pow(u, 1.15);
}

function patchAlpha(p: Patch, g: number): number {
  const local = Math.max(0, Math.min(1, (g - p.delay) / Math.max(0.08, 1 - p.delay)));
  return local * p.strength * (0.25 + g * 0.75);
}

function maybeSpawnDew(g: number, now: number) {
  if (g < 0.22) return;
  if (dews.length > 140) return;
  if (Math.random() > 0.08 + g * 0.12) return;
  dews.push({
    x: Math.random() * w,
    y: Math.random() * h,
    r: 0.8 + Math.random() * 2.8 * (0.4 + g),
    born: now,
  });
}

function drawDew(d: Dew, now: number) {
  if (!ctx) return;
  const age = Math.min(1, (now - d.born) / 900);
  const a = 0.15 + age * 0.35;
  const g = ctx.createRadialGradient(d.x - d.r * 0.25, d.y - d.r * 0.3, 0, d.x, d.y, d.r);
  g.addColorStop(0, `rgba(255,255,255,${a})`);
  g.addColorStop(0.55, `rgba(220,230,236,${a * 0.35})`);
  g.addColorStop(1, "rgba(200,210,220,0)");
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.ellipse(d.x, d.y, d.r * 0.9, d.r, 0, 0, Math.PI * 2);
  ctx.fill();
}

function tick(now: number) {
  if (!running || !ctx) return;
  if (paused) {
    raf = requestAnimationFrame(tick);
    return;
  }
  const g = progress(now);

  ctx.clearRect(0, 0, w, h);

  // Base milk veil — cold glass haze (cool gray-white, not blue rain tint)
  const base = 0.04 + g * 0.52;
  ctx.fillStyle = `rgba(232, 236, 238, ${base})`;
  ctx.fillRect(0, 0, w, h);

  // Mottled condensation patches (breath / humidity blooming)
  for (const p of patches) {
    const a = patchAlpha(p, g);
    if (a < 0.01) continue;
    const r = p.r * (0.85 + g * 0.35);
    const grd = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, r);
    grd.addColorStop(0, `rgba(246, 248, 249, ${a * 0.95})`);
    grd.addColorStop(0.45, `rgba(228, 232, 235, ${a * 0.55})`);
    grd.addColorStop(1, "rgba(210, 216, 220, 0)");
    ctx.fillStyle = grd;
    ctx.beginPath();
    ctx.arc(p.x, p.y, r, 0, Math.PI * 2);
    ctx.fill();
  }

  maybeSpawnDew(g, now);
  for (const d of dews) drawDew(d, now);

  // Late frost-like uniform film so the desktop becomes unreadable
  const film = Math.pow(g, 1.4) * 0.78;
  if (film > 0.01) {
    ctx.fillStyle = `rgba(240, 243, 245, ${film})`;
    ctx.fillRect(0, 0, w, h);
  }

  raf = requestAnimationFrame(tick);
}

export function startRainGlass(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  seedPatches();
  dews = [];
  startedAt = performance.now();
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopRainGlass() {
  running = false;
  cancelAnimationFrame(raf);
  patches = [];
  dews = [];
  if (ctx && canvas) {
    ctx.clearRect(0, 0, w, h);
  }
}

export function setRainGlassPaused(held: boolean) {
  paused = held;
}

export function onRainGlassResize() {
  if (!running) return;
  resize();
  seedPatches();
}

export function isRainGlassRunning() {
  return running;
}
