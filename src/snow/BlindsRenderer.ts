/**
 * Venetian blinds: drop down while open (see-through), then slowly close.
 */

let canvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let running = false;
let paused = false;
let raf = 0;
let w = 0;
let h = 0;
let startedAt = 0;
let pausedAccum = 0;
let pauseBegan = 0;

/** Drop the open stack from the top. */
const DROP_SECONDS = 2.4;
/** Brief hold fully open before closing. */
const HOLD_SECONDS = 0.6;
/** Slow close — intentionally longer than drop. */
const CLOSE_SECONDS = 9.5;

const RAIL_H = 14;
const SLAT_GAP = 3;
const SLAT_OPEN_H = 4;
/** Target closed height per slot (covers gap). */
const SLAT_CLOSED_H = 28;

function easeOutCubic(t: number): number {
  const u = Math.min(1, Math.max(0, t));
  return 1 - Math.pow(1 - u, 3);
}

function easeInOutCubic(t: number): number {
  const u = Math.min(1, Math.max(0, t));
  return u < 0.5 ? 4 * u * u * u : 1 - Math.pow(-2 * u + 2, 3) / 2;
}

function resize() {
  if (!canvas || !ctx) return;
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function elapsedSeconds(now: number): number {
  return Math.max(0, (now - startedAt - pausedAccum) / 1000);
}

function phases(t: number): { drop: number; close: number } {
  if (t <= DROP_SECONDS) {
    return { drop: easeOutCubic(t / DROP_SECONDS), close: 0 };
  }
  const afterDrop = t - DROP_SECONDS;
  if (afterDrop <= HOLD_SECONDS) {
    return { drop: 1, close: 0 };
  }
  const closeT = (afterDrop - HOLD_SECONDS) / CLOSE_SECONDS;
  return { drop: 1, close: easeInOutCubic(closeT) };
}

function slotPitch(): number {
  return SLAT_CLOSED_H + SLAT_GAP;
}

function drawRail() {
  if (!ctx) return;
  const g = ctx.createLinearGradient(0, 0, 0, RAIL_H);
  g.addColorStop(0, "#6a6258");
  g.addColorStop(0.45, "#4e4840");
  g.addColorStop(1, "#3a352f");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, RAIL_H);
  ctx.fillStyle = "rgba(255,255,255,0.12)";
  ctx.fillRect(0, 0, w, 2);
  ctx.fillStyle = "rgba(0,0,0,0.28)";
  ctx.fillRect(0, RAIL_H - 2, w, 2);
}

function drawSlat(y: number, slatH: number, close: number, index: number) {
  if (!ctx || slatH < 0.5) return;
  const tilt = close; // 0 open (thin edge), 1 closed (face-on)
  const shade = 0.55 + tilt * 0.28 + (index % 2) * 0.03;
  const base = Math.floor(92 * shade);
  const g = Math.floor(86 * shade);
  const b = Math.floor(72 * shade);

  const grad = ctx.createLinearGradient(0, y, 0, y + slatH);
  grad.addColorStop(0, `rgb(${base + 28},${g + 24},${b + 18})`);
  grad.addColorStop(0.18, `rgb(${base + 10},${g + 8},${b + 6})`);
  grad.addColorStop(0.55, `rgb(${base},${g},${b})`);
  grad.addColorStop(1, `rgb(${Math.max(30, base - 22)},${Math.max(28, g - 20)},${Math.max(24, b - 16)})`);
  ctx.fillStyle = grad;
  ctx.fillRect(0, y, w, slatH);

  // top highlight — stronger when more face-on
  ctx.fillStyle = `rgba(255,245,230,${0.08 + tilt * 0.16})`;
  ctx.fillRect(0, y, w, Math.max(1, slatH * 0.12));

  // bottom lip shadow
  ctx.fillStyle = `rgba(0,0,0,${0.18 + tilt * 0.22})`;
  ctx.fillRect(0, y + slatH - Math.max(1, slatH * 0.14), w, Math.max(1, slatH * 0.14));

  // subtle vertical edge bevels
  ctx.fillStyle = `rgba(255,255,255,${0.04 + tilt * 0.05})`;
  ctx.fillRect(0, y, 3, slatH);
  ctx.fillStyle = `rgba(0,0,0,${0.1 + tilt * 0.08})`;
  ctx.fillRect(w - 3, y, 3, slatH);
}

function tick(now: number) {
  if (!running || !ctx) return;
  if (paused) {
    raf = requestAnimationFrame(tick);
    return;
  }

  const { drop, close } = phases(elapsedSeconds(now));
  const pitch = slotPitch();
  const openH = SLAT_OPEN_H;
  const closedH = SLAT_CLOSED_H;
  const slatH = openH + (closedH - openH) * close;
  // while open, pitch stays so gaps show; as close increases height eats the gap
  const visibleBottom = RAIL_H + (h - RAIL_H) * drop;
  const maxSlots = Math.ceil((h - RAIL_H) / pitch) + 2;

  ctx.clearRect(0, 0, w, h);

  // ambient dimming as blinds close
  if (close > 0.02) {
    ctx.fillStyle = `rgba(18, 16, 14, ${close * 0.55})`;
    ctx.fillRect(0, 0, w, h);
  }

  drawRail();

  // cords / pull hint on the right
  if (drop > 0.05) {
    const cordX = w - 22;
    ctx.strokeStyle = "rgba(60,52,44,0.55)";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.moveTo(cordX, RAIL_H);
    ctx.lineTo(cordX, Math.min(visibleBottom, h * 0.35 + drop * h * 0.2));
    ctx.stroke();
  }

  for (let i = 0; i < maxSlots; i++) {
    const y = RAIL_H + i * pitch;
    if (y >= visibleBottom) break;
    const drawH = Math.min(slatH, visibleBottom - y);
    if (drawH <= 0) continue;
    drawSlat(y, drawH, close, i);
  }

  // soft shadow under the descending stack edge
  if (drop < 0.999) {
    const edge = visibleBottom;
    const shade = ctx.createLinearGradient(0, edge, 0, edge + 18);
    shade.addColorStop(0, "rgba(0,0,0,0.28)");
    shade.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = shade;
    ctx.fillRect(0, edge, w, 18);
  }

  raf = requestAnimationFrame(tick);
}

export function startBlinds(el: HTMLCanvasElement) {
  canvas = el;
  ctx = el.getContext("2d", { alpha: true });
  if (!ctx) return;
  resize();
  pausedAccum = 0;
  pauseBegan = 0;
  startedAt = performance.now();
  if (running) return;
  running = true;
  raf = requestAnimationFrame(tick);
}

export function stopBlinds() {
  running = false;
  cancelAnimationFrame(raf);
  paused = false;
  pausedAccum = 0;
  if (ctx) ctx.clearRect(0, 0, w, h);
}

export function setBlindsPaused(held: boolean) {
  if (held === paused) return;
  if (held) {
    pauseBegan = performance.now();
    paused = true;
  } else {
    if (pauseBegan) pausedAccum += performance.now() - pauseBegan;
    pauseBegan = 0;
    paused = false;
  }
}

export function onBlindsResize() {
  if (!running) return;
  resize();
}

export function isBlindsRunning() {
  return running;
}
