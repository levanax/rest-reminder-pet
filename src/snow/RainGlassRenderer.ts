/**
 * Car-glass condensation: uneven milky patches, clearer center, still dew.
 * Matches a windshield fogging from the inside — not a flat white sheet.
 */

type Dew = {
  x: number;
  y: number;
  r: number;
};

let canvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let noise: HTMLCanvasElement | null = null;
let running = false;
let paused = false;
let raf = 0;
let w = 0;
let h = 0;
let startedAt = 0;
let dews: Dew[] = [];
const FOG_SECONDS = 80;

function resize() {
  if (!canvas || !ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

/** Low-res value noise, stretched — looks like breath on glass, not circles. */
function buildNoise() {
  const nw = 96;
  const nh = 54;
  const grid: number[][] = [];
  for (let y = 0; y <= nh; y++) {
    grid[y] = [];
    for (let x = 0; x <= nw; x++) grid[y][x] = Math.random();
  }
  const smooth = (t: number) => t * t * (3 - 2 * t);
  noise = document.createElement("canvas");
  noise.width = nw * 4;
  noise.height = nh * 4;
  const nctx = noise.getContext("2d")!;
  const img = nctx.createImageData(noise.width, noise.height);
  const data = img.data;
  for (let y = 0; y < noise.height; y++) {
    for (let x = 0; x < noise.width; x++) {
      const fx = (x / noise.width) * nw;
      const fy = (y / noise.height) * nh;
      const x0 = Math.floor(fx);
      const y0 = Math.floor(fy);
      const tx = smooth(fx - x0);
      const ty = smooth(fy - y0);
      const v00 = grid[y0][x0];
      const v10 = grid[y0][Math.min(nw, x0 + 1)];
      const v01 = grid[Math.min(nh, y0 + 1)][x0];
      const v11 = grid[Math.min(nh, y0 + 1)][Math.min(nw, x0 + 1)];
      const v0 = v00 + (v10 - v00) * tx;
      const v1 = v01 + (v11 - v01) * tx;
      let v = v0 + (v1 - v0) * ty;
      // second octave
      const fx2 = fx * 2.3;
      const fy2 = fy * 2.3;
      const a0 = Math.floor(fx2) % (nw + 1);
      const b0 = Math.floor(fy2) % (nh + 1);
      v = v * 0.72 + grid[b0][a0] * 0.28;
      const i = (y * noise.width + x) * 4;
      const c = 210 + v * 45;
      data[i] = c;
      data[i + 1] = c + 2;
      data[i + 2] = c + 4;
      // alpha: keep dark valleys almost clear so patches stay see-through
      data[i + 3] = Math.floor(Math.pow(v, 1.35) * 210);
    }
  }
  nctx.putImageData(img, 0, 0);
}

function progress(now: number): number {
  const t = Math.max(0, (now - startedAt) / 1000);
  const u = Math.min(1, t / FOG_SECONDS);
  return u * u * (3 - 2 * u);
}

function spawnDews(g: number) {
  const want = Math.floor(g * g * 90);
  while (dews.length < want) {
    // prefer edges, like real windshield fog
    const edge = Math.random();
    let x = Math.random() * w;
    let y = Math.random() * h;
    if (edge < 0.55) {
      if (Math.random() < 0.5) x = Math.random() * w * 0.22;
      else x = w * (0.78 + Math.random() * 0.22);
    }
    if (edge > 0.4 && edge < 0.75) {
      if (Math.random() < 0.5) y = Math.random() * h * 0.2;
      else y = h * (0.8 + Math.random() * 0.2);
    }
    dews.push({ x, y, r: 0.6 + Math.random() * 2.2 });
  }
}

function drawDew(d: Dew) {
  if (!ctx) return;
  ctx.beginPath();
  ctx.ellipse(d.x, d.y, d.r * 0.85, d.r, 0, 0, Math.PI * 2);
  const g = ctx.createRadialGradient(d.x - d.r * 0.3, d.y - d.r * 0.35, 0, d.x, d.y, d.r);
  g.addColorStop(0, "rgba(255,255,255,0.85)");
  g.addColorStop(0.35, "rgba(230,236,240,0.35)");
  g.addColorStop(1, "rgba(200,210,216,0)");
  ctx.fillStyle = g;
  ctx.fill();
}

function tick(now: number) {
  if (!running || !ctx || !noise) return;
  if (paused) {
    raf = requestAnimationFrame(tick);
    return;
  }
  const g = progress(now);
  ctx.clearRect(0, 0, w, h);

  // mottled film — alpha grows, valleys stay clearer
  ctx.save();
  ctx.globalAlpha = 0.08 + g * 0.82;
  ctx.drawImage(noise, 0, 0, w, h);
  ctx.restore();

  // edge-heavier second pass (windshield fogs from the rim)
  ctx.save();
  ctx.globalAlpha = g * 0.55;
  ctx.drawImage(noise, -w * 0.04, -h * 0.04, w * 1.08, h * 1.08);
  const clear = 1 - g * 0.92;
  const hole = ctx.createRadialGradient(w * 0.5, h * 0.48, Math.min(w, h) * 0.08, w * 0.5, h * 0.48, Math.max(w, h) * 0.62);
  hole.addColorStop(0, `rgba(0,0,0,${clear})`);
  hole.addColorStop(0.55, `rgba(0,0,0,${clear * 0.4})`);
  hole.addColorStop(1, "rgba(0,0,0,0)");
  ctx.globalCompositeOperation = "destination-out";
  ctx.fillStyle = hole;
  ctx.fillRect(0, 0, w, h);
  ctx.restore();

  spawnDews(g);
  if (g > 0.25) {
    ctx.save();
    ctx.globalAlpha = Math.min(1, (g - 0.25) * 1.4);
    for (const d of dews) drawDew(d);
    ctx.restore();
  }

  raf = requestAnimationFrame(tick);
}

export function startRainGlass(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  buildNoise();
  dews = [];
  startedAt = performance.now();
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopRainGlass() {
  running = false;
  cancelAnimationFrame(raf);
  dews = [];
  noise = null;
  if (ctx) ctx.clearRect(0, 0, w, h);
}

export function setRainGlassPaused(held: boolean) {
  paused = held;
}

export function onRainGlassResize() {
  if (!running) return;
  resize();
  buildNoise();
}

export function isRainGlassRunning() {
  return running;
}
