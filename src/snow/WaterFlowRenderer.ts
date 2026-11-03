/**
 * Water curtain: thin streams from the top edge that slowly thicken and multiply.
 */

type Stream = {
  x: number;
  width: number;
  speed: number;
  phase: number;
  wobble: number;
  len: number;
  y: number;
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
let streams: Stream[] = [];
const RAMP_SECONDS = 75;

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

function spawn(g: number): Stream {
  const thin = g < 0.35;
  return {
    x: Math.random() * w,
    width: thin ? 0.6 + Math.random() * 1.4 : 1.8 + g * 10 + Math.random() * 6,
    speed: 90 + Math.random() * 140 + g * 80,
    phase: Math.random() * Math.PI * 2,
    wobble: (Math.random() - 0.5) * (0.8 + g * 2.2),
    len: h * (0.35 + Math.random() * 0.55 + g * 0.2),
    y: -20 - Math.random() * h * 0.25,
  };
}

function targetCount(g: number): number {
  const area = Math.max(1, w * h);
  const light = Math.min(28, Math.max(6, Math.floor(area / 90000)));
  const heavy = Math.min(90, Math.max(22, Math.floor(area / 28000)));
  return Math.floor(light * (1 - g) + heavy * g);
}

function syncCount(g: number) {
  const n = targetCount(g);
  while (streams.length < n) streams.push(spawn(g));
  if (streams.length > n) streams.length = n;
}

function drawStream(s: Stream, t: number, g: number) {
  if (!ctx) return;
  const sway = Math.sin(t * 1.1 + s.phase) * s.wobble;
  const x = s.x + sway;
  const top = s.y;
  const bottom = Math.min(h + 40, s.y + s.len);
  const half = s.width * 0.5;

  const grad = ctx.createLinearGradient(x, top, x, bottom);
  grad.addColorStop(0, "rgba(210,230,245,0)");
  grad.addColorStop(0.08, `rgba(190,220,240,${0.12 + g * 0.18})`);
  grad.addColorStop(0.45, `rgba(160,200,230,${0.22 + g * 0.28})`);
  grad.addColorStop(0.85, `rgba(140,185,220,${0.18 + g * 0.22})`);
  grad.addColorStop(1, "rgba(120,170,210,0)");

  ctx.beginPath();
  ctx.moveTo(x - half * 0.35, top);
  ctx.quadraticCurveTo(x - half * 0.7 + sway * 0.3, (top + bottom) * 0.5, x - half, bottom);
  ctx.lineTo(x + half, bottom);
  ctx.quadraticCurveTo(x + half * 0.7 + sway * 0.3, (top + bottom) * 0.5, x + half * 0.35, top);
  ctx.closePath();
  ctx.fillStyle = grad;
  ctx.fill();

  // bright core
  ctx.beginPath();
  ctx.moveTo(x - half * 0.15, top + 8);
  ctx.lineTo(x + half * 0.15, top + 8);
  ctx.lineTo(x + half * 0.25, bottom - 10);
  ctx.lineTo(x - half * 0.25, bottom - 10);
  ctx.closePath();
  ctx.fillStyle = `rgba(255,255,255,${0.08 + g * 0.16})`;
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
  // cool wash, still see-through
  ctx.fillStyle = `rgba(35, 55, 75, ${0.03 + g * 0.12})`;
  ctx.fillRect(0, 0, w, h);

  // soft puddle at bottom grows with intensity
  if (g > 0.05) {
    const puddle = ctx.createLinearGradient(0, h * (0.82 - g * 0.08), 0, h);
    puddle.addColorStop(0, "rgba(90,140,180,0)");
    puddle.addColorStop(1, `rgba(70,120,160,${0.08 + g * 0.22})`);
    ctx.fillStyle = puddle;
    ctx.fillRect(0, h * 0.75, w, h * 0.25);
  }

  for (const s of streams) {
    s.y += s.speed * dt;
    if (s.y > h + 40) {
      const next = spawn(g);
      s.x = next.x;
      s.width = next.width;
      s.speed = next.speed;
      s.phase = next.phase;
      s.wobble = next.wobble;
      s.len = next.len;
      s.y = next.y;
      continue;
    }
    // gradually thicken existing streams
    const targetW = 0.6 + g * 12 + (s.phase % 1) * 4;
    s.width += (targetW - s.width) * Math.min(1, dt * 0.35);
    drawStream(s, t, g);
  }

  raf = requestAnimationFrame(tick);
}

export function startWaterFlow(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  streams = [];
  syncCount(0);
  startedAt = performance.now();
  lastT = startedAt;
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopWaterFlow() {
  running = false;
  cancelAnimationFrame(raf);
  streams = [];
  if (ctx) ctx.clearRect(0, 0, w, h);
}

export function setWaterFlowPaused(held: boolean) {
  paused = held;
  if (!held) lastT = performance.now();
}

export function onWaterFlowResize() {
  if (!running) return;
  resize();
}

export function isWaterFlowRunning() {
  return running;
}
