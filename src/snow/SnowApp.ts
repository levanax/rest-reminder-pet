import { listen } from "@tauri-apps/api/event";

type Layer = "far" | "mid" | "near";

type Flake = {
  x: number;
  y: number;
  size: number;
  speed: number;
  drift: number;
  swing: number;
  phase: number;
  spin: number;
  rot: number;
  opacity: number;
  layer: Layer;
  kind: "dot" | "crystal";
  sprite: HTMLCanvasElement;
};

const canvas = document.getElementById("snow") as HTMLCanvasElement;
const ctx = canvas.getContext("2d", { alpha: true })!;

let flakes: Flake[] = [];
let running = false;
let raf = 0;
let w = 0;
let h = 0;
let wind = 0;
let windTarget = 0;
let lastT = 0;

const spriteCache = new Map<string, HTMLCanvasElement>();

function resize() {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

/** Soft circular powder flake (distant snow). */
function makeDotSprite(radius: number, glow: number): HTMLCanvasElement {
  const pad = Math.ceil(radius * 3 + glow * 2);
  const c = document.createElement("canvas");
  c.width = pad * 2;
  c.height = pad * 2;
  const g = c.getContext("2d")!;
  const grd = g.createRadialGradient(pad, pad, 0, pad, pad, radius + glow);
  grd.addColorStop(0, "rgba(255,255,255,0.95)");
  grd.addColorStop(0.35, "rgba(235,245,255,0.75)");
  grd.addColorStop(0.7, "rgba(200,220,245,0.25)");
  grd.addColorStop(1, "rgba(200,220,245,0)");
  g.fillStyle = grd;
  g.beginPath();
  g.arc(pad, pad, radius + glow, 0, Math.PI * 2);
  g.fill();
  return c;
}

/** 6-fold crystal snowflake. */
function makeCrystalSprite(size: number): HTMLCanvasElement {
  const pad = Math.ceil(size * 2.4);
  const c = document.createElement("canvas");
  c.width = pad * 2;
  c.height = pad * 2;
  const g = c.getContext("2d")!;
  g.translate(pad, pad);

  // soft bloom behind crystal
  const bloom = g.createRadialGradient(0, 0, 0, 0, 0, size * 1.35);
  bloom.addColorStop(0, "rgba(240,248,255,0.55)");
  bloom.addColorStop(0.55, "rgba(210,230,255,0.18)");
  bloom.addColorStop(1, "rgba(210,230,255,0)");
  g.fillStyle = bloom;
  g.beginPath();
  g.arc(0, 0, size * 1.35, 0, Math.PI * 2);
  g.fill();

  g.strokeStyle = "rgba(255,255,255,0.92)";
  g.fillStyle = "rgba(245,250,255,0.88)";
  g.lineWidth = Math.max(0.8, size * 0.11);
  g.lineCap = "round";
  g.lineJoin = "round";
  g.shadowColor = "rgba(180,210,255,0.65)";
  g.shadowBlur = size * 0.35;

  for (let i = 0; i < 6; i++) {
    g.save();
    g.rotate((Math.PI / 3) * i);

    // main arm
    g.beginPath();
    g.moveTo(0, 0);
    g.lineTo(0, -size);
    g.stroke();

    // side branches
    const b1 = size * 0.42;
    const b2 = size * 0.72;
    g.beginPath();
    g.moveTo(0, -b1);
    g.lineTo(-size * 0.28, -b1 - size * 0.18);
    g.moveTo(0, -b1);
    g.lineTo(size * 0.28, -b1 - size * 0.18);
    g.moveTo(0, -b2);
    g.lineTo(-size * 0.22, -b2 - size * 0.14);
    g.moveTo(0, -b2);
    g.lineTo(size * 0.22, -b2 - size * 0.14);
    g.stroke();

    // tip diamond
    g.beginPath();
    g.moveTo(0, -size);
    g.lineTo(-size * 0.12, -size + size * 0.16);
    g.lineTo(0, -size + size * 0.28);
    g.lineTo(size * 0.12, -size + size * 0.16);
    g.closePath();
    g.fill();

    g.restore();
  }

  // center hub
  g.shadowBlur = 0;
  g.beginPath();
  g.arc(0, 0, Math.max(1.2, size * 0.14), 0, Math.PI * 2);
  g.fill();

  return c;
}

function getSprite(kind: "dot" | "crystal", size: number): HTMLCanvasElement {
  const key = `${kind}-${size.toFixed(1)}`;
  let s = spriteCache.get(key);
  if (!s) {
    s = kind === "crystal" ? makeCrystalSprite(size) : makeDotSprite(size * 0.55, size * 0.9);
    spriteCache.set(key, s);
  }
  return s;
}

function pickLayer(): Layer {
  const r = Math.random();
  if (r < 0.45) return "far";
  if (r < 0.8) return "mid";
  return "near";
}

function makeFlake(fromTop: boolean): Flake {
  const layer = pickLayer();
  let size: number;
  let speed: number;
  let opacity: number;
  let kind: "dot" | "crystal";

  if (layer === "far") {
    size = 1.4 + Math.random() * 2.2;
    speed = 0.35 + Math.random() * 0.55;
    opacity = 0.25 + Math.random() * 0.25;
    kind = "dot";
  } else if (layer === "mid") {
    size = 3.2 + Math.random() * 3.5;
    speed = 0.7 + Math.random() * 1.1;
    opacity = 0.45 + Math.random() * 0.3;
    kind = Math.random() < 0.55 ? "crystal" : "dot";
  } else {
    size = 5.5 + Math.random() * 5.5;
    speed = 1.2 + Math.random() * 1.6;
    opacity = 0.65 + Math.random() * 0.3;
    kind = Math.random() < 0.85 ? "crystal" : "dot";
  }

  // snap size for sprite cache reuse
  size = Math.round(size * 2) / 2;

  return {
    x: Math.random() * w,
    y: fromTop ? -30 - Math.random() * h * 0.4 : Math.random() * h,
    size,
    speed,
    drift: 0.15 + Math.random() * 0.7,
    swing: 0.6 + Math.random() * 1.4,
    phase: Math.random() * Math.PI * 2,
    spin: (Math.random() - 0.5) * 0.04,
    rot: Math.random() * Math.PI * 2,
    opacity,
    layer,
    kind,
    sprite: getSprite(kind, size),
  };
}

function densify() {
  const area = Math.max(1, w * h);
  // a bit denser for a fuller snowfall, still capped for perf
  const target = Math.min(520, Math.max(120, Math.floor(area / 14000)));
  while (flakes.length < target) flakes.push(makeFlake(true));
  if (flakes.length > target) flakes.length = target;
}

function drawFlake(f: Flake) {
  const spr = f.sprite;
  const hw = spr.width / 2;
  const hh = spr.height / 2;
  ctx.save();
  ctx.translate(f.x, f.y);
  if (f.kind === "crystal") ctx.rotate(f.rot);
  ctx.globalAlpha = f.opacity;
  ctx.drawImage(spr, -hw, -hh);
  ctx.restore();
}

function startSnow() {
  resize();
  flakes = [];
  densify();
  for (let i = 0; i < flakes.length; i++) flakes[i] = makeFlake(false);
  wind = 0;
  windTarget = (Math.random() - 0.5) * 0.6;
  lastT = performance.now();

  if (running) return;
  running = true;

  const tick = (now: number) => {
    if (!running) return;
    const dt = Math.min(0.05, (now - lastT) / 1000);
    lastT = now;
    const t = now / 1000;

    // gentle gusts
    if (Math.random() < 0.01) windTarget = (Math.random() - 0.5) * 1.2;
    wind += (windTarget - wind) * Math.min(1, dt * 0.8);

    ctx.clearRect(0, 0, w, h);

    // draw far -> near for depth
    const order: Layer[] = ["far", "mid", "near"];
    for (const layer of order) {
      for (const f of flakes) {
        if (f.layer !== layer) continue;
        f.phase += dt * f.swing;
        f.rot += f.spin;
        f.y += f.speed * (1 + Math.sin(t * 0.7 + f.phase) * 0.08);
        f.x += wind * (0.4 + f.size * 0.08) + Math.sin(t * 0.9 + f.phase) * f.drift;

        if (f.y - f.size > h + 20) {
          f.y = -20 - Math.random() * 60;
          f.x = Math.random() * w;
        }
        if (f.x < -40) f.x = w + 40;
        if (f.x > w + 40) f.x = -40;

        drawFlake(f);
      }
    }

    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);
}

function stopSnow() {
  running = false;
  cancelAnimationFrame(raf);
  flakes = [];
  ctx.clearRect(0, 0, w, h);
}

window.addEventListener("resize", () => {
  if (!running) return;
  resize();
  densify();
});

async function main() {
  await listen<{ active: boolean }>("snow-event", (e) => {
    if (e.payload?.active) startSnow();
    else stopSnow();
  });
}

void main();
