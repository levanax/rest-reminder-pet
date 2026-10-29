/**
 * Rain on window glass: fine short streaks → fat drops with a long wet trail
 * and a bright bulb at the leading edge. Glass stays see-through.
 */

type Drop = {
  x: number;
  y: number;
  len: number;
  width: number;
  speed: number;
  mass: number;
  wobble: number;
  phase: number;
};

let canvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let running = false;
let paused = false;
let raf = 0;
let w = 0;
let h = 0;
let lastT = 0;
let startedAt = 0;
let drops: Drop[] = [];
const RAMP_SECONDS = 70;

function resize() {
  if (!canvas || !ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function intensity(now: number): number {
  const t = Math.max(0, (now - startedAt) / 1000);
  const u = Math.min(1, t / RAMP_SECONDS);
  return u * u * (3 - 2 * u);
}

function spawn(g: number): Drop {
  const bias = Math.random();
  const mass = g < 0.2 ? bias * 0.22 : Math.min(1, bias * (0.25 + g * 0.9));
  const fine = mass < 0.4;
  return {
    x: Math.random() * w,
    y: -20 - Math.random() * h * 0.4,
    len: fine ? 10 + Math.random() * 22 : 40 + mass * 140,
    width: fine ? 0.45 + Math.random() * 0.7 : 1.4 + mass * 3.4,
    speed: fine ? 160 + Math.random() * 220 : 70 + Math.random() * 90,
    mass,
    wobble: (Math.random() - 0.5) * (fine ? 0.4 : 1.6),
    phase: Math.random() * Math.PI * 2,
  };
}

function targetCount(g: number): number {
  const area = Math.max(1, w * h);
  const drizzle = Math.min(260, Math.max(40, Math.floor(area / 14000)));
  const heavy = Math.min(70, Math.max(12, Math.floor(area / 42000)));
  return Math.floor(drizzle * (1 - g) + heavy * g);
}

function syncCount(g: number) {
  const n = targetCount(g);
  while (drops.length < n) drops.push(spawn(g));
  if (drops.length > n) drops.length = n;
}

function drawDrop(d: Drop, t: number) {
  if (!ctx) return;
  const sway = Math.sin(t * 1.4 + d.phase) * d.wobble;
  const x = d.x + sway;
  const headY = d.y;
  const tailY = d.y - d.len;
  const half = d.width * 0.5;

  // wet trail: thin at the tail, slightly wider toward the head
  ctx.beginPath();
  ctx.moveTo(x - half * 0.25, tailY);
  ctx.lineTo(x + half * 0.25, tailY);
  ctx.lineTo(x + half, headY);
  ctx.lineTo(x - half, headY);
  ctx.closePath();
  const trail = ctx.createLinearGradient(x, tailY, x, headY);
  trail.addColorStop(0, "rgba(190,210,225,0)");
  trail.addColorStop(0.35, `rgba(200,218,230,${0.08 + d.mass * 0.12})`);
  trail.addColorStop(1, `rgba(220,232,242,${0.18 + d.mass * 0.28})`);
  ctx.fillStyle = trail;
  ctx.fill();

  if (d.mass < 0.35) return;

  const br = d.width * (1.15 + d.mass * 0.9);
  const bulb = ctx.createRadialGradient(x - br * 0.25, headY - br * 0.35, br * 0.05, x, headY, br);
  bulb.addColorStop(0, "rgba(255,255,255,0.95)");
  bulb.addColorStop(0.28, `rgba(230,240,248,${0.55 + d.mass * 0.3})`);
  bulb.addColorStop(0.7, `rgba(180,200,216,${0.18 + d.mass * 0.15})`);
  bulb.addColorStop(1, "rgba(160,180,196,0)");
  ctx.fillStyle = bulb;
  ctx.beginPath();
  ctx.ellipse(x, headY, br * 0.72, br * 1.05, 0, 0, Math.PI * 2);
  ctx.fill();
}

function tick(now: number) {
  if (!running || !ctx) return;
  if (paused) {
    lastT = now;
    raf = requestAnimationFrame(tick);
    return;
  }
  const dt = Math.min(0.05, (now - lastT) / 1000);
  lastT = now;
  const g = intensity(now);
  const t = now / 1000;
  syncCount(g);

  ctx.clearRect(0, 0, w, h);
  // wet glass: slight cool darkening, still see-through
  ctx.fillStyle = `rgba(40, 55, 70, ${0.04 + g * 0.1})`;
  ctx.fillRect(0, 0, w, h);

  for (const d of drops) {
    d.y += d.speed * dt;
    if (d.y - d.len > h + 30) {
      const next = spawn(g);
      d.x = next.x;
      d.y = next.y;
      d.len = next.len;
      d.width = next.width;
      d.speed = next.speed;
      d.mass = next.mass;
      d.wobble = next.wobble;
      d.phase = next.phase;
      continue;
    }
    drawDrop(d, t);
  }

  raf = requestAnimationFrame(tick);
}

export function startRainStreak(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  drops = [];
  syncCount(0);
  startedAt = performance.now();
  lastT = startedAt;
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopRainStreak() {
  running = false;
  cancelAnimationFrame(raf);
  drops = [];
  if (ctx) ctx.clearRect(0, 0, w, h);
}

export function setRainStreakPaused(held: boolean) {
  paused = held;
  if (!held) lastT = performance.now();
}

export function onRainStreakResize() {
  if (!running) return;
  resize();
}

export function isRainStreakRunning() {
  return running;
}
