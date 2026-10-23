import { invoke } from "@tauri-apps/api/core";
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

/** Horizontal resolution of the bottom snow pile heightmap (px per column). */
const PILE_STEP = 6;
/** Max pile height as fraction of screen — high enough to bury the pet. */
const PILE_MAX_FRAC = 0.42;
/** Max pile height in px (hard cap). */
const PILE_MAX_PX = 360;

type MonitorRectPhysical = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type SnowEventPayload = {
  active: boolean;
  window?: { width: number; height: number };
  monitors?: MonitorRectPhysical[];
};

/** One physical monitor's snow pile, in CSS pixels relative to the snow window. */
type ScreenPile = {
  x: number;
  y: number;
  w: number;
  h: number;
  pile: Float32Array;
};

let flakes: Flake[] = [];
let running = false;
let paused = false;
let raf = 0;
let w = 0;
let h = 0;
let wind = 0;
let windTarget = 0;
let lastT = 0;
/** Per-monitor settled snow along each monitor's own bottom edge. */
let screens: ScreenPile[] = [];
/** Last physical layout from Rust (for remap on resize). */
let lastLayout: { window: { width: number; height: number }; monitors: MonitorRectPhysical[] } | null =
  null;

const spriteCache = new Map<string, HTMLCanvasElement>();

function maxPileHeightFor(screenH: number) {
  return Math.min(PILE_MAX_PX, screenH * PILE_MAX_FRAC);
}

function pileColsFor(screenW: number) {
  return Math.max(1, Math.ceil(screenW / PILE_STEP) + 1);
}

function makeScreen(x: number, y: number, sw: number, sh: number, old?: Float32Array): ScreenPile {
  const cols = pileColsFor(sw);
  const pile = new Float32Array(cols);
  if (old?.length) {
    const scale = old.length / cols;
    for (let i = 0; i < cols; i++) {
      const j = Math.min(old.length - 1, Math.floor(i * scale));
      pile[i] = old[j];
    }
  }
  return { x, y, w: sw, h: sh, pile };
}

function fallbackScreens(): ScreenPile[] {
  return [makeScreen(0, 0, w, h)];
}

/**
 * After physical→CSS mapping, force the virtual-desktop bbox onto this canvas.
 * RDP / DPI races often leave innerWidth/Height out of sync with Rust's physical
 * size, so pile bottoms float mid-screen or sit below the visible edge.
 */
function alignScreensToCanvas(preservePiles: Float32Array[]) {
  if (!screens.length || w <= 0 || h <= 0) {
    screens = fallbackScreens();
    return;
  }
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const s of screens) {
    minX = Math.min(minX, s.x);
    minY = Math.min(minY, s.y);
    maxX = Math.max(maxX, s.x + s.w);
    maxY = Math.max(maxY, s.y + s.h);
  }
  const layoutW = maxX - minX;
  const layoutH = maxY - minY;
  if (!(layoutW > 1) || !(layoutH > 1)) {
    screens = fallbackScreens();
    return;
  }

  const needsX = Math.abs(minX) > 1.5 || Math.abs(maxX - w) > 1.5;
  const needsY = Math.abs(minY) > 1.5 || Math.abs(maxY - h) > 1.5;
  if (!needsX && !needsY) return;

  const sx = w / layoutW;
  const sy = h / layoutH;
  screens = screens.map((s, idx) =>
    makeScreen(
      (s.x - minX) * sx,
      (s.y - minY) * sy,
      s.w * sx,
      s.h * sy,
      preservePiles[idx] ?? s.pile,
    ),
  );
}

/** Map physical monitor rects from Rust into CSS space of this window. */
function applyMonitorLayout(
  physicalWindow: { width: number; height: number },
  physicalMonitors: MonitorRectPhysical[],
  preserve = false,
) {
  lastLayout = { window: physicalWindow, monitors: physicalMonitors };
  if (!physicalMonitors.length || physicalWindow.width <= 0 || physicalWindow.height <= 0) {
    screens = fallbackScreens();
    return;
  }
  const sx = w / physicalWindow.width;
  const sy = h / physicalWindow.height;
  const prev = preserve ? screens : [];
  const prevPiles = prev.map((s) => s.pile);
  screens = physicalMonitors.map((m, idx) =>
    makeScreen(m.x * sx, m.y * sy, m.width * sx, m.height * sy, prev[idx]?.pile),
  );
  if (!screens.length) {
    screens = fallbackScreens();
    return;
  }
  alignScreensToCanvas(preserve ? prevPiles : screens.map((s) => s.pile));
}

function resetPiles() {
  if (lastLayout) {
    applyMonitorLayout(lastLayout.window, lastLayout.monitors, false);
  } else {
    screens = fallbackScreens();
  }
}

function pileIndex(pile: Float32Array, localX: number) {
  const i = Math.floor(localX / PILE_STEP);
  return Math.max(0, Math.min(pile.length - 1, i));
}

function pileHeightAt(screen: ScreenPile, localX: number) {
  const pile = screen.pile;
  if (!pile.length) return 0;
  const f = localX / PILE_STEP;
  const i0 = Math.max(0, Math.min(pile.length - 1, Math.floor(f)));
  const i1 = Math.max(0, Math.min(pile.length - 1, i0 + 1));
  const t = f - i0;
  return pile[i0] * (1 - t) + pile[i1] * t;
}

/** Add mass to a monitor pile and lightly smooth neighbors so mounds look soft. */
function depositSnow(screen: ScreenPile, localX: number, amount: number) {
  const pile = screen.pile;
  if (!pile.length) return;
  const maxH = maxPileHeightFor(screen.h);
  const i = pileIndex(pile, localX);
  pile[i] = Math.min(maxH, pile[i] + amount);
  for (const d of [-1, 1]) {
    const j = i + d;
    if (j < 0 || j >= pile.length) continue;
    pile[j] = Math.min(maxH, pile[j] + amount * 0.35);
  }
}

/** Gentle avalanche: tall columns spill into shorter neighbors (per monitor). */
function settlePiles(dt: number) {
  const rate = Math.min(1, dt * 2.2);
  for (const screen of screens) {
    const pile = screen.pile;
    if (pile.length < 2) continue;
    const maxH = maxPileHeightFor(screen.h);
    for (let i = 0; i < pile.length - 1; i++) {
      const diff = pile[i] - pile[i + 1];
      if (Math.abs(diff) < 1.2) continue;
      const move = diff * 0.18 * rate;
      pile[i] -= move;
      pile[i + 1] += move;
    }
    for (let i = 0; i < pile.length; i++) {
      pile[i] = Math.min(maxH, Math.max(0, pile[i]));
    }
  }
}

/**
 * Find the topmost monitor ground under (x,y) that the flake has reached.
 * Picks the highest (smallest Y) landing line among monitors covering this X.
 */
function landingAt(x: number, y: number, flakeSize: number): {
  screen: ScreenPile;
  localX: number;
  ground: number;
} | null {
  let best: { screen: ScreenPile; localX: number; ground: number } | null = null;
  for (const screen of screens) {
    if (x < screen.x || x >= screen.x + screen.w) continue;
    const localX = x - screen.x;
    const ground = screen.y + screen.h - pileHeightAt(screen, localX) - flakeSize * 0.35;
    if (y < ground) continue;
    if (!best || ground < best.ground) {
      best = { screen, localX, ground };
    }
  }
  return best;
}

function drawPiles() {
  for (const screen of screens) {
    const pile = screen.pile;
    if (!pile.length) continue;
    const maxH = maxPileHeightFor(screen.h);
    if (maxH <= 0) continue;
    const bottom = screen.y + screen.h;

    // soft blue-white mound along this monitor's bottom
    ctx.save();
    ctx.beginPath();
    ctx.moveTo(screen.x, bottom + 2);
    for (let i = 0; i < pile.length; i++) {
      const x = screen.x + i * PILE_STEP;
      const py = bottom - pile[i];
      ctx.lineTo(x, py);
    }
    ctx.lineTo(screen.x + screen.w + 2, bottom - pile[pile.length - 1]);
    ctx.lineTo(screen.x + screen.w + 2, bottom + 2);
    ctx.closePath();

    // denser fill so deep snow can fully bury the cat underneath
    const grd = ctx.createLinearGradient(0, bottom - maxH, 0, bottom);
    grd.addColorStop(0, "rgba(248,252,255,0.82)");
    grd.addColorStop(0.4, "rgba(236,245,255,0.94)");
    grd.addColorStop(1, "rgba(220,234,250,0.98)");
    ctx.fillStyle = grd;
    ctx.fill();

    // sparkle rim along the crest
    ctx.beginPath();
    ctx.moveTo(screen.x, bottom - pile[0]);
    for (let i = 1; i < pile.length; i++) {
      ctx.lineTo(screen.x + i * PILE_STEP, bottom - pile[i]);
    }
    ctx.strokeStyle = "rgba(255,255,255,0.55)";
    ctx.lineWidth = 1.4;
    ctx.stroke();
    ctx.restore();
  }
}

function resize() {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  w = window.innerWidth;
  h = window.innerHeight;
  canvas.width = Math.floor(w * dpr);
  canvas.height = Math.floor(h * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (lastLayout) {
    applyMonitorLayout(lastLayout.window, lastLayout.monitors, true);
  } else if (!screens.length) {
    screens = fallbackScreens();
  } else {
    // keep single full-canvas pile if no layout yet
    screens = [makeScreen(0, 0, w, h, screens[0]?.pile)];
  }
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

/** Apply a layout update without wiping flakes / piles (RDP resync). */
function syncLayoutFromPayload(payload: SnowEventPayload, preservePiles: boolean) {
  resize();
  if (payload.window && payload.monitors?.length) {
    applyMonitorLayout(payload.window, payload.monitors, preservePiles);
  } else if (lastLayout) {
    applyMonitorLayout(lastLayout.window, lastLayout.monitors, preservePiles);
  } else {
    screens = fallbackScreens();
  }
  densify();
}

/** Re-read CSS size for a few frames — WebView often lags set_size under RDP. */
function scheduleLayoutResync() {
  const delays = [0, 50, 120, 280, 600];
  for (const ms of delays) {
    window.setTimeout(() => {
      if (!running || !lastLayout) return;
      const prevW = w;
      const prevH = h;
      resize();
      if (w !== prevW || h !== prevH || screens.length === 0) {
        applyMonitorLayout(lastLayout.window, lastLayout.monitors, true);
        densify();
      } else {
        // Size stable but bottoms may still be wrong relative to canvas
        alignScreensToCanvas(screens.map((s) => s.pile));
      }
    }, ms);
  }
}

function startSnow(payload?: SnowEventPayload) {
  resize();
  if (payload?.window && payload.monitors?.length) {
    applyMonitorLayout(payload.window, payload.monitors, false);
  } else {
    resetPiles();
  }
  flakes = [];
  densify();
  for (let i = 0; i < flakes.length; i++) flakes[i] = makeFlake(false);
  wind = 0;
  windTarget = (Math.random() - 0.5) * 0.6;
  lastT = performance.now();
  scheduleLayoutResync();

  if (running) return;
  running = true;

  const tick = (now: number) => {
    if (!running) return;
    if (paused) {
      lastT = now;
      raf = requestAnimationFrame(tick);
      return;
    }
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

        if (f.x < -40) f.x = w + 40;
        if (f.x > w + 40) f.x = -40;

        // land on the topmost monitor pile under this column
        const land = landingAt(f.x, f.y, f.size);
        if (land) {
          const mass =
            (f.layer === "near" ? 2.4 : f.layer === "mid" ? 1.5 : 0.75) *
            (0.7 + f.size * 0.12);
          depositSnow(land.screen, land.localX, mass);
          f.y = -20 - Math.random() * 60;
          f.x = Math.random() * w;
          continue;
        }

        // fell through a gap between monitors — recycle
        if (f.y > h + 40) {
          f.y = -20 - Math.random() * 60;
          f.x = Math.random() * w;
          continue;
        }

        drawFlake(f);
      }
    }

    settlePiles(dt);
    drawPiles();

    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);
}

function stopSnow() {
  running = false;
  cancelAnimationFrame(raf);
  flakes = [];
  lastLayout = null;
  screens = [];
  ctx.clearRect(0, 0, w, h);
}

let resizeRefreshTimer = 0;
window.addEventListener("resize", () => {
  if (!running) return;
  resize();
  densify();
  // Ask Rust to re-measure monitors after the OS settles (RDP resolution flip).
  window.clearTimeout(resizeRefreshTimer);
  resizeRefreshTimer = window.setTimeout(() => {
    if (!running) return;
    void invoke("refresh_snow_layout_cmd").catch(() => {
      if (lastLayout) applyMonitorLayout(lastLayout.window, lastLayout.monitors, true);
    });
  }, 120);
});

async function main() {
  await listen<{ held?: boolean }>("session-hold", (e) => {
    paused = !!e.payload?.held;
    if (paused) lastT = performance.now();
  });
  await listen<SnowEventPayload>("snow-event", (e) => {
    const payload = e.payload;
    if (!payload?.active) {
      stopSnow();
      return;
    }
    // Already snowing: layout-only refresh (delayed RDP resync must not reset piles).
    if (running) {
      syncLayoutFromPayload(payload, true);
      scheduleLayoutResync();
      return;
    }
    startSnow(payload);
  });
}

void main();
