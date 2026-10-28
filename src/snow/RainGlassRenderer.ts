/** Full-screen rain-on-glass + fog that densifies until nearly opaque. */

type Drop = {
  x: number;
  y: number;
  r: number;
  vy: number;
  life: number;
  maxLife: number;
};

type Streak = {
  x: number;
  y: number;
  len: number;
  speed: number;
  width: number;
  opacity: number;
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
let streaks: Streak[] = [];
/** Fog opacity target ramp: ~0 → ~0.92 over fogSeconds. */
const FOG_SECONDS = 75;

function resize() {
  if (!canvas || !ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function spawnDrop(): Drop {
  return {
    x: Math.random() * w,
    y: Math.random() * h * 0.85,
    r: 1.2 + Math.random() * 3.5,
    vy: 0,
    life: 0,
    maxLife: 2.5 + Math.random() * 5,
  };
}

function spawnStreak(): Streak {
  return {
    x: Math.random() * w,
    y: -20 - Math.random() * 80,
    len: 18 + Math.random() * 42,
    speed: 220 + Math.random() * 320,
    width: 1 + Math.random() * 1.6,
    opacity: 0.12 + Math.random() * 0.28,
  };
}

function densify() {
  const area = Math.max(1, w * h);
  const dropTarget = Math.min(180, Math.max(40, Math.floor(area / 18000)));
  const streakTarget = Math.min(90, Math.max(25, Math.floor(area / 28000)));
  while (drops.length < dropTarget) drops.push(spawnDrop());
  if (drops.length > dropTarget) drops.length = dropTarget;
  while (streaks.length < streakTarget) streaks.push(spawnStreak());
  if (streaks.length > streakTarget) streaks.length = streakTarget;
}

function fogOpacity(now: number): number {
  const t = Math.max(0, (now - startedAt) / 1000);
  const u = Math.min(1, t / FOG_SECONDS);
  // ease-in so early minutes stay readable, late nearly opaque
  return 0.08 + Math.pow(u, 1.35) * 0.86;
}

function drawDrop(d: Drop) {
  if (!ctx) return;
  const g = ctx.createRadialGradient(d.x, d.y, 0, d.x, d.y, d.r);
  g.addColorStop(0, "rgba(255,255,255,0.55)");
  g.addColorStop(0.45, "rgba(210,230,245,0.28)");
  g.addColorStop(1, "rgba(180,210,230,0)");
  ctx.fillStyle = g;
  ctx.beginPath();
  ctx.ellipse(d.x, d.y, d.r * 0.85, d.r, 0, 0, Math.PI * 2);
  ctx.fill();
}

function drawStreak(s: Streak) {
  if (!ctx) return;
  const grd = ctx.createLinearGradient(s.x, s.y, s.x, s.y + s.len);
  grd.addColorStop(0, `rgba(230,240,255,0)`);
  grd.addColorStop(0.35, `rgba(220,235,250,${s.opacity})`);
  grd.addColorStop(1, `rgba(200,220,240,0)`);
  ctx.strokeStyle = grd;
  ctx.lineWidth = s.width;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(s.x, s.y);
  ctx.lineTo(s.x + s.width * 0.4, s.y + s.len);
  ctx.stroke();
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

  ctx.clearRect(0, 0, w, h);

  const fog = fogOpacity(now);
  // mist veil
  ctx.fillStyle = `rgba(198, 214, 224, ${fog * 0.55})`;
  ctx.fillRect(0, 0, w, h);
  ctx.fillStyle = `rgba(236, 242, 246, ${fog * 0.5})`;
  ctx.fillRect(0, 0, w, h);

  for (const s of streaks) {
    s.y += s.speed * dt;
    if (s.y > h + 40) {
      Object.assign(s, spawnStreak());
      s.y = -30;
    }
    drawStreak(s);
  }

  for (const d of drops) {
    d.life += dt;
    // occasional drip
    if (d.life > d.maxLife * 0.55 && Math.random() < 0.02) {
      d.vy = 40 + Math.random() * 80;
    }
    d.y += d.vy * dt;
    d.vy *= 0.98;
    if (d.life > d.maxLife || d.y > h + 10) {
      Object.assign(d, spawnDrop());
      continue;
    }
    drawDrop(d);
  }

  // final soft fog on top so screen ends nearly unreadable
  ctx.fillStyle = `rgba(245, 248, 250, ${fog * 0.72})`;
  ctx.fillRect(0, 0, w, h);

  raf = requestAnimationFrame(tick);
}

export function startRainGlass(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  drops = [];
  streaks = [];
  densify();
  startedAt = performance.now();
  lastT = startedAt;
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopRainGlass() {
  running = false;
  cancelAnimationFrame(raf);
  drops = [];
  streaks = [];
  if (ctx && canvas) {
    ctx.clearRect(0, 0, w, h);
  }
}

export function setRainGlassPaused(held: boolean) {
  paused = held;
  if (!held) lastT = performance.now();
}

export function onRainGlassResize() {
  if (!running) return;
  resize();
  densify();
}

export function isRainGlassRunning() {
  return running;
}
