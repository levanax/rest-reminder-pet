/**
 * Rain on glass: starts as fine drizzle, intensifies to fat sliding droplets.
 */

type Drop = {
  x: number;
  y: number;
  /** trail length in px */
  len: number;
  /** horizontal thickness */
  width: number;
  speed: number;
  opacity: number;
  /** slight zigzag phase */
  phase: number;
  /** 0 = fine drizzle, 1 = fat slider */
  mass: number;
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
/** Seconds from drizzle → heavy. */
const RAMP_SECONDS = 65;

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
  // linger in drizzle, then thicken
  return Math.pow(u, 0.85);
}

function targetCount(g: number): number {
  const area = Math.max(1, w * h);
  const base = Math.floor(area / 32000);
  const lo = Math.max(18, base);
  const hi = Math.min(220, Math.floor(area / 9000));
  return Math.floor(lo + (hi - lo) * g);
}

function spawnDrop(g: number, fromTop: boolean): Drop {
  // mass bias grows with intensity
  const roll = Math.random();
  const mass =
    g < 0.25
      ? roll * 0.25
      : g < 0.55
        ? roll * (0.35 + g * 0.4)
        : Math.min(1, roll * (0.45 + g * 0.7));

  const fine = mass < 0.45;
  return {
    x: Math.random() * w,
    y: fromTop ? -10 - Math.random() * 80 : Math.random() * h * 0.3,
    len: fine ? 8 + Math.random() * 18 : 28 + mass * 70 + Math.random() * 40,
    width: fine ? 0.6 + Math.random() * 0.9 : 1.6 + mass * 3.2,
    speed: fine
      ? 140 + Math.random() * 180
      : 90 + Math.random() * 120 + mass * 80,
    opacity: fine ? 0.12 + Math.random() * 0.2 : 0.28 + mass * 0.45,
    phase: Math.random() * Math.PI * 2,
    mass,
  };
}

function densify(g: number) {
  const target = targetCount(g);
  while (drops.length < target) drops.push(spawnDrop(g, true));
  if (drops.length > target) drops.length = target;
}

function drawDrop(d: Drop, t: number) {
  if (!ctx) return;
  const wobble = Math.sin(t * 2.2 + d.phase) * (0.4 + d.mass * 1.2);
  const x0 = d.x + wobble;
  const y0 = d.y;
  const x1 = d.x + wobble * 0.3 + d.width * 0.15;
  const y1 = d.y + d.len;

  const grd = ctx.createLinearGradient(x0, y0, x1, y1);
  if (d.mass < 0.45) {
    grd.addColorStop(0, `rgba(230,240,250,0)`);
    grd.addColorStop(0.4, `rgba(210,225,240,${d.opacity})`);
    grd.addColorStop(1, `rgba(190,210,230,0)`);
  } else {
    // fat bead: brighter head, long translucent trail
    grd.addColorStop(0, `rgba(245,250,255,0)`);
    grd.addColorStop(0.55, `rgba(220,235,248,${d.opacity * 0.55})`);
    grd.addColorStop(0.88, `rgba(235,245,255,${d.opacity})`);
    grd.addColorStop(1, `rgba(255,255,255,${Math.min(0.9, d.opacity + 0.15)})`);
  }

  ctx.strokeStyle = grd;
  ctx.lineWidth = d.width;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x0, y0);
  ctx.lineTo(x1, y1);
  ctx.stroke();

  if (d.mass >= 0.55) {
    // leading droplet bulb
    const br = d.width * (1.1 + d.mass * 0.8);
    const bulb = ctx.createRadialGradient(x1, y1, 0, x1, y1, br);
    bulb.addColorStop(0, `rgba(255,255,255,${0.55 + d.mass * 0.25})`);
    bulb.addColorStop(0.5, `rgba(210,230,245,${0.25 * d.opacity})`);
    bulb.addColorStop(1, "rgba(180,200,220,0)");
    ctx.fillStyle = bulb;
    ctx.beginPath();
    ctx.ellipse(x1, y1, br * 0.75, br, 0, 0, Math.PI * 2);
    ctx.fill();
  }
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

  densify(g);

  ctx.clearRect(0, 0, w, h);

  // subtle wet glass tint that grows with rain
  ctx.fillStyle = `rgba(160, 185, 205, ${0.04 + g * 0.12})`;
  ctx.fillRect(0, 0, w, h);

  for (const d of drops) {
    d.y += d.speed * dt * (0.85 + d.mass * 0.35);
    // fat drops slow slightly and pick up a bit of mass feel
    if (d.mass > 0.5 && Math.random() < 0.01 * g) {
      d.len = Math.min(h * 0.45, d.len + 4);
      d.width = Math.min(6.5, d.width + 0.05);
    }
    if (d.y > h + d.len + 20) {
      Object.assign(d, spawnDrop(g, true));
      continue;
    }
    drawDrop(d, t);
  }

  // occasional splash of new fat drops when heavy
  if (g > 0.55 && Math.random() < 0.04 * g && drops.length < targetCount(g) + 8) {
    drops.push(spawnDrop(g, true));
  }

  raf = requestAnimationFrame(tick);
}

export function startRainStreak(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  drops = [];
  densify(0);
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
  if (ctx && canvas) {
    ctx.clearRect(0, 0, w, h);
  }
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
