import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type ActionDef = { fps: number; frames: string[] };
type Pack = {
  name: string;
  manifest: {
    frameSize: { w: number; h: number };
    actions: Record<string, ActionDef>;
  };
  frames: Record<string, string>;
};

type PetEvent = {
  phase: string;
  action: string | null;
  characterPack: string;
};

type Pose = {
  y: number;
  x: number;
  rot: number;
  sx: number;
  sy: number;
  opacity: number;
  frameT: number;
};

const DEFAULT_DISPLAY_MAX_PX = 96;

type AppConfigDto = {
  displayMaxPx?: number;
};

const root = document.getElementById("app")!;
root.innerHTML = `
  <div id="stage" style="position:relative;width:100%;height:100%;background:transparent;">
    <img id="pet" alt="" draggable="false"
      style="position:absolute;left:50%;top:-160px;width:96px;height:96px;transform:translateX(-50%);
             image-rendering:auto;pointer-events:none;opacity:0;transform-origin:50% 0%;" />
  </div>
`;

const img = document.getElementById("pet") as HTMLImageElement;
let pack: Pack | null = null;
let loadedPackId: string | null = null;
let moveRaf: number | null = null;
let currentAction: string | null = null;
let lastFrameFile: string | null = null;
let frameSize = { w: 128, h: 128 };
let displayMaxPx = DEFAULT_DISPLAY_MAX_PX;
let displayW = DEFAULT_DISPLAY_MAX_PX;
let displayH = DEFAULT_DISPLAY_MAX_PX;

function clampDisplayMaxPx(n: number) {
  if (!Number.isFinite(n) || n <= 0) return DEFAULT_DISPLAY_MAX_PX;
  return Math.min(192, Math.max(48, Math.round(n)));
}

function applyDisplaySize() {
  const m = Math.max(frameSize.w, frameSize.h) || 1;
  displayW = Math.round((frameSize.w * displayMaxPx) / m);
  displayH = Math.round((frameSize.h * displayMaxPx) / m);
  img.style.width = `${displayW}px`;
  img.style.height = `${displayH}px`;
}

function setDisplayMaxPx(px: number) {
  displayMaxPx = clampDisplayMaxPx(px);
  applyDisplaySize();
}

async function loadDisplayConfig() {
  try {
    const cfg = await invoke<AppConfigDto>("get_config");
    setDisplayMaxPx(cfg.displayMaxPx ?? DEFAULT_DISPLAY_MAX_PX);
  } catch {
    setDisplayMaxPx(DEFAULT_DISPLAY_MAX_PX);
  }
}

function setOrigin(origin: string) {
  img.style.transformOrigin = origin;
}

/** Fully above the screen (on the cliff top platform). */
function yAboveCliff() {
  return -displayH - 8;
}

/** Peeking over the cliff rim: cancel typical top pad (~112/512). */
function yCliffPeek() {
  return -Math.round(displayH * 0.22);
}

/** Standing on the bottom ledge / platform. */
function yBottomLedge() {
  return window.innerHeight - Math.round(displayH * 0.92);
}

async function ensurePack(name: string) {
  if (pack && loadedPackId === name) return pack;
  pack = await invoke<Pack>("get_pack", { name });
  loadedPackId = name;
  frameSize = pack.manifest.frameSize;
  applyDisplaySize();
  return pack;
}

function clearTimers() {
  if (moveRaf != null) {
    window.cancelAnimationFrame(moveRaf);
    moveRaf = null;
  }
}

function applyPose(pose: Pose, frames: string[]) {
  img.style.top = `${pose.y}px`;
  img.style.opacity = String(pose.opacity);
  img.style.transform = `translateX(calc(-50% + ${pose.x}px)) rotate(${pose.rot}deg) scale(${pose.sx}, ${pose.sy})`;
  if (!frames.length || !pack) return;
  const idx = Math.min(
    frames.length - 1,
    Math.max(0, Math.floor(pose.frameT * frames.length * 0.999)),
  );
  const file = frames[idx];
  if (file && file !== lastFrameFile) {
    img.src = pack.frames[file] || "";
    lastFrameFile = file;
  }
}

function lerp(a: number, b: number, t: number) {
  return a + (b - a) * t;
}

function easeOutCubic(t: number) {
  return 1 - Math.pow(1 - t, 3);
}

function easeOutQuad(t: number) {
  return 1 - (1 - t) * (1 - t);
}

function easeInQuad(t: number) {
  return t * t;
}

function easeInOutQuad(t: number) {
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
}

function smoothstep(t: number) {
  const x = Math.min(1, Math.max(0, t));
  return x * x * (3 - 2 * x);
}

/** Progress with brief pauses near given centers (claw pushes). */
function pausedProgress(t: number, pauses: number[], pauseWidth = 0.06): number {
  const samples = 64;
  let acc = 0;
  const weights: number[] = [];
  for (let i = 0; i <= samples; i++) {
    const u = i / samples;
    let w = 1;
    for (const p of pauses) {
      const d = Math.abs(u - p);
      if (d < pauseWidth) {
        w *= d / pauseWidth;
      }
    }
    weights.push(Math.max(0.05, w));
    if (i > 0) acc += weights[i];
  }
  const target = t * acc;
  let sum = 0;
  for (let i = 1; i <= samples; i++) {
    sum += weights[i];
    if (sum >= target) {
      const prev = sum - weights[i];
      const local = (target - prev) / weights[i];
      return (i - 1 + local) / samples;
    }
  }
  return 1;
}

function runTimeline(
  durationMs: number,
  frames: string[],
  sample: (t: number) => Pose,
  done: () => void,
) {
  clearTimers();
  lastFrameFile = null;
  const start = performance.now();
  const tick = (now: number) => {
    const t = Math.min(1, (now - start) / durationMs);
    applyPose(sample(t), frames);
    if (t < 1) {
      moveRaf = window.requestAnimationFrame(tick);
    } else {
      moveRaf = null;
      done();
    }
  };
  moveRaf = window.requestAnimationFrame(tick);
}

function actionFrames(action: string): string[] {
  return pack?.manifest.actions[action]?.frames ?? [];
}

function hopFrames(): string[] {
  const jump = actionFrames("jumpUp");
  if (jump.length) return jump;
  return actionFrames("fall");
}

/** Prefer walk frames for pacing; fall back to crawl, then jump/fall. */
function walkFrames(): string[] {
  const walk = actionFrames("walk");
  if (walk.length) return walk;
  const crawl = actionFrames("crawl");
  if (crawl.length) return crawl;
  return hopFrames();
}

async function playAction(action: string) {
  if (!pack) return;
  if (action === "sneakPeek") {
    await playSneakPeek();
    return;
  }
  if (action === "jumpUp") {
    await playJumpUp();
    return;
  }
  if (action === "crawl") {
    await playCrawl();
    return;
  }
  if (action === "lookDown") {
    await playLookDownOnce();
    return;
  }
  if (action === "anxiousHop") {
    await playAnxiousHop();
    return;
  }
  if (action === "happyClimb") {
    await playHappyClimb();
    return;
  }
  if (action === "fall") {
    await playFall();
    return;
  }
  await invoke("notify_action_done", { action });
}

async function playJumpUp() {
  if (!pack) return;
  const def =
    pack.manifest.actions.jumpUp ??
    pack.manifest.actions.fall ??
    pack.manifest.actions.happyClimb;
  if (!def?.frames?.length) {
    await invoke("notify_action_done", { action: "jumpUp" });
    return;
  }
  currentAction = "jumpUp";
  setOrigin("50% 85%");
  const ledge = yBottomLedge();
  const above = yAboveCliff();
  // Cat-like double load: shallow sink → tiny rebound → deep crouch
  const crouchShallow = ledge + displayH * 0.035;
  const crouchDeep = ledge + displayH * 0.1;
  // Push-off asymmetry (which way the hind legs shove)
  const driftSign = Math.random() < 0.5 ? -1 : 1;
  const driftAmp = displayW * (0.1 + Math.random() * 0.06);

  /**
   * Ballistic rise in screen space (y down): fast leave, then gravity-like slowdown.
   * s(u)=1-(1-u)^k maps u∈[0,1] → progress toward cliff; k>2 = punchy launch.
   */
  const ballistic = (u: number, k = 2.35) => 1 - Math.pow(1 - u, k);

  runTimeline(
    1900,
    def.frames,
    (t) => {
      let y: number;
      let x = 0;
      let sx = 1;
      let sy = 1;
      let rot = 0;
      let opacity = 1;
      let frameT = t;

      if (t < 0.22) {
        // 1) Anticipation — double crouch load
        const u = t / 0.22;
        if (u < 0.35) {
          const p = u / 0.35;
          y = lerp(ledge, crouchShallow, easeOutQuad(p));
          sx = lerp(1, 1.1, p);
          sy = lerp(1, 0.88, p);
          rot = lerp(0, 4, p);
          frameT = p * 0.08;
        } else if (u < 0.5) {
          // tiny rebound (weight shift)
          const p = (u - 0.35) / 0.15;
          y = lerp(crouchShallow, ledge + displayH * 0.012, p);
          sx = lerp(1.1, 1.04, p);
          sy = lerp(0.88, 0.95, p);
          rot = lerp(4, 2, p);
          frameT = 0.08 + p * 0.04;
        } else {
          // deep coil
          const p = (u - 0.5) / 0.5;
          const e = easeInOutQuad(p);
          y = lerp(ledge + displayH * 0.012, crouchDeep, e);
          sx = lerp(1.04, 1.22, e);
          sy = lerp(0.95, 0.72, e);
          rot = lerp(2, 8, e);
          frameT = 0.12 + e * 0.06;
        }
      } else if (t < 0.36) {
        // 2) Explosive takeoff — stretch + nose up
        const u = (t - 0.22) / 0.14;
        const e = easeOutCubic(u);
        // clear ~28% of the climb in the burst
        const burstEnd = lerp(crouchDeep, above, 0.28);
        y = lerp(crouchDeep, burstEnd, e);
        x = driftSign * driftAmp * 0.15 * e;
        sx = lerp(1.22, 0.82, e);
        sy = lerp(0.72, 1.28, e);
        rot = lerp(8, -16, e);
        frameT = 0.18 + e * 0.22;
      } else if (t < 0.86) {
        // 3) Airborne ballistic arc toward cliff
        const u = (t - 0.36) / 0.5;
        const burstEnd = lerp(crouchDeep, above, 0.28);
        const s = ballistic(u, 2.4);
        y = lerp(burstEnd, above, s);
        // lateral drift peaks mid-flight then settles (asymmetric shove)
        x = driftSign * driftAmp * Math.sin(u * Math.PI * 0.92);
        // stretch relaxes; micro flutter from air resistance
        const flutter = Math.sin(u * Math.PI * 5.5) * 0.025 * (1 - u);
        sx = lerp(0.82, 1.02, easeOutQuad(u)) + flutter;
        sy = lerp(1.28, 0.96, easeOutQuad(u)) - flutter;
        // rotate with "velocity": steep early, flatten near rim
        const velFeel = Math.pow(1 - u, 1.4);
        rot = lerp(-16, -3, easeOutQuad(u)) - velFeel * 4;
        frameT = 0.4 + s * 0.42;
      } else {
        // 4) Scramble over cliff rim — fully off-screen, then brief fade
        const u = (t - 0.86) / 0.14;
        // slight overshoot deeper above, then settle
        const overshoot = above - displayH * 0.08 * Math.sin(Math.min(1, u) * Math.PI);
        y = overshoot;
        x = driftSign * driftAmp * 0.12 * (1 - u);
        sx = lerp(1.02, 1, u);
        sy = lerp(0.96, 1.04, Math.sin(u * Math.PI));
        rot = lerp(-3, 2, u);
        frameT = 0.82 + u * 0.18;
        if (t >= 0.96) {
          opacity = lerp(1, 0, (t - 0.96) / 0.04);
        }
      }

      return {
        y,
        x,
        rot,
        sx,
        sy,
        opacity,
        frameT: Math.min(1, Math.max(0, frameT)),
      };
    },
    async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "jumpUp" });
    },
  );
}

async function playCrawl() {
  if (!pack) return;
  const frames = actionFrames("crawl");
  if (!frames.length) {
    await invoke("notify_action_done", { action: "crawl" });
    return;
  }
  currentAction = "crawl";
  setOrigin("50% 0%");
  const from = yAboveCliff();
  const to = yCliffPeek();

  runTimeline(
    1400,
    frames,
    (t) => {
      const p = smoothstep(pausedProgress(t, [0.28, 0.52, 0.76], 0.07));
      const sway = Math.sin(p * Math.PI * 3);
      return {
        y: lerp(from, to, p),
        x: sway * 6,
        rot: sway * 4,
        sx: 1,
        sy: 1 + Math.abs(Math.sin(p * Math.PI * 3)) * 0.03,
        opacity: 1,
        frameT: p,
      };
    },
    async () => {
      clearTimers();
      await invoke("notify_action_done", { action: "crawl" });
    },
  );
}

async function playLookDownOnce() {
  if (!pack) return;
  const frames = actionFrames("lookDown");
  if (!frames.length) {
    await invoke("notify_action_done", { action: "lookDown" });
    return;
  }
  currentAction = "lookDown";
  setOrigin("50% 0%");
  const y = yCliffPeek();

  runTimeline(
    1000,
    frames,
    (t) => {
      const sway = Math.sin(t * Math.PI * 2) * (1 - t);
      const breath = Math.sin(t * Math.PI * 3);
      return {
        y,
        x: sway * 3,
        rot: sway * 2,
        sx: 1,
        sy: lerp(0.96, 1, 0.5 + 0.5 * breath),
        opacity: 1,
        frameT: Math.min(0.8, t),
      };
    },
    async () => {
      clearTimers();
      await invoke("notify_action_done", { action: "lookDown" });
    },
  );
}

async function playHappyClimb() {
  if (!pack) return;
  const frames = actionFrames("happyClimb");
  if (!frames.length) {
    await invoke("notify_action_done", { action: "happyClimb" });
    return;
  }
  currentAction = "happyClimb";
  setOrigin("50% 0%");
  const parsed = parseFloat(img.style.top);
  const from = Number.isFinite(parsed) ? parsed : yCliffPeek();
  const to = yAboveCliff();

  runTimeline(
    1200,
    frames,
    (t) => {
      const p = easeInOutQuad(pausedProgress(t, [0.35, 0.65], 0.07));
      const sway = Math.sin(p * Math.PI * 2.5);
      const y = lerp(from, to, p);
      // fade only after fully above cliff
      let opacity = 1;
      if (y <= to && t > 0.85) {
        opacity = lerp(1, 0, (t - 0.85) / 0.15);
      }
      return {
        y,
        x: sway * 6,
        rot: sway * 5,
        sx: 1,
        sy: 1 + Math.abs(sway) * 0.04,
        opacity,
        frameT: p,
      };
    },
    async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "happyClimb" });
    },
  );
}

async function playFall() {
  if (!pack) return;
  const frames = actionFrames("fall");
  if (!frames.length) {
    await invoke("notify_action_done", { action: "fall" });
    return;
  }
  currentAction = "fall";
  setOrigin("50% 40%");
  const parsed = parseFloat(img.style.top);
  const from = Number.isFinite(parsed) ? parsed : yCliffPeek();
  const to = yBottomLedge();

  runTimeline(
    1100,
    frames,
    (t) => {
      const g = easeInQuad(t);
      const wobble = Math.sin(t * Math.PI * 2.2) * (1 - t) * 6;
      const fade = t > 0.82 ? (t - 0.82) / 0.18 : 0;
      return {
        y: lerp(from, to, g),
        x: wobble,
        rot: lerp(0, 25, g) + Math.sin(t * Math.PI * 3) * 4,
        sx: lerp(1, 0.97, g),
        sy: lerp(1, 1.06, g),
        opacity: 1 - fade,
        frameT: g,
      };
    },
    async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "fall" });
    },
  );
}

async function playSneakPeek() {
  if (!pack) return;
  // Force lookDown frames (standing sneakPeek poses don't fit cliff peek)
  const frames = actionFrames("lookDown");
  if (!frames.length) {
    await invoke("notify_action_done", { action: "sneakPeek" });
    return;
  }
  currentAction = "sneakPeek";
  setOrigin("50% 0%");
  const from = yAboveCliff();
  const peek = yCliffPeek();
  const hide = yAboveCliff();
  const total = 2800;
  const outEnd = 900 / total;
  const holdEnd = (900 + 1200) / total;

  runTimeline(
    total,
    frames,
    (t) => {
      let y: number;
      let frameT: number;
      if (t < outEnd) {
        const u = t / outEnd;
        y = lerp(from, peek, easeOutQuad(u));
        frameT = Math.min(1, u * 0.75);
      } else if (t < holdEnd) {
        const u = (t - outEnd) / (holdEnd - outEnd);
        y = peek;
        frameT = 0.5 + Math.sin(u * Math.PI) * 0.35;
      } else {
        const u = (t - holdEnd) / (1 - holdEnd);
        y = lerp(peek, hide, easeInQuad(u));
        frameT = Math.max(0, 0.5 - u * 0.5);
      }

      const breathT = performance.now() / 900;
      const breath = Math.sin(breathT);
      const micro = t >= outEnd && t < holdEnd;
      return {
        y,
        x: micro ? Math.sin(breathT * 0.7) * 3 : 0,
        rot: micro ? Math.sin(breathT * 0.55) * 2.5 : 0,
        sx: 1 - (micro ? breath * 0.012 : 0),
        sy: micro ? lerp(0.92, 1, 0.5 + 0.5 * breath) : 1,
        opacity: t >= 0.97 ? lerp(1, 0, (t - 0.97) / 0.03) : 1,
        frameT: Math.min(1, Math.max(0, frameT)),
      };
    },
    async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "sneakPeek" });
    },
  );
}

/** Drop from cliff peek to bottom ledge, then anxious walk loop. */
async function playAnxiousHop() {
  if (!pack) return;
  const dropFrames = hopFrames();
  const frames = walkFrames();
  if (!frames.length && !dropFrames.length) {
    await invoke("notify_action_done", { action: "anxiousHop" });
    return;
  }
  currentAction = "anxiousHop";
  setOrigin("50% 85%");
  const parsed = parseFloat(img.style.top);
  const fromY = Number.isFinite(parsed) ? parsed : yCliffPeek();
  const toY = yBottomLedge();
  const needDrop = fromY < toY - displayH * 0.2;
  const landFrames = dropFrames.length ? dropFrames : frames;

  const startLoopAndNotify = () => {
    void playAnxiousWalkLoop(frames.length ? frames : landFrames);
    void invoke("notify_action_done", { action: "anxiousHop" });
  };

  if (!needDrop) {
    startLoopAndNotify();
    return;
  }

  runTimeline(
    650,
    landFrames,
    (t) => {
      const g = easeInQuad(t);
      return {
        y: lerp(fromY, toY, g),
        x: Math.sin(t * Math.PI * 2) * 8,
        rot: lerp(0, 18, g) + Math.sin(t * Math.PI * 3) * 4,
        sx: lerp(1, 0.95, g),
        sy: lerp(1, 1.08, g),
        opacity: 1,
        frameT: 0.35 + g * 0.5,
      };
    },
    () => {
      if (currentAction !== "anxiousHop") return;
      startLoopAndNotify();
    },
  );
}

/** Pace left/right on the bottom ledge (anxious walk, not hop). */
function playAnxiousWalkLoop(frames: string[]) {
  clearTimers();
  lastFrameFile = null;
  currentAction = "anxiousHop";
  setOrigin("50% 85%");

  const halfSpan = Math.min(220, window.innerWidth * 0.32);
  const fps =
    pack?.manifest.actions.walk?.fps ??
    pack?.manifest.actions.crawl?.fps ??
    8;
  // Keep horizontal drift mild so legs (driven by fps) don't look like kicks.
  const speed = 42;
  let x = 0;
  let dir = Math.random() < 0.5 ? -1 : 1;
  let framePhase = 0;
  let lastNow = performance.now();

  const tick = (now: number) => {
    if (currentAction !== "anxiousHop") return;

    const dt = Math.min(0.05, (now - lastNow) / 1000);
    lastNow = now;

    x += dir * speed * dt;
    if (x > halfSpan) {
      x = halfSpan;
      dir = -1;
    } else if (x < -halfSpan) {
      x = -halfSpan;
      dir = 1;
    }

    framePhase += dt * fps;
    const n = Math.max(1, frames.length);
    const frameT = (framePhase % n) / n;
    const ledge = yBottomLedge();

    applyPose(
      {
        y: ledge,
        x,
        rot: 0,
        sx: dir < 0 ? -1 : 1,
        sy: 1,
        opacity: 1,
        frameT,
      },
      frames,
    );
    moveRaf = window.requestAnimationFrame(tick);
  };
  moveRaf = window.requestAnimationFrame(tick);
}

async function onPetEvent(ev: PetEvent) {
  if (!ev.action) {
    img.style.opacity = "0";
    clearTimers();
    currentAction = null;
    return;
  }
  await ensurePack(ev.characterPack);
  if (ev.action === currentAction && ev.phase === "observing") {
    return;
  }
  if (ev.phase === "observing" && ev.action === "anxiousHop") {
    if (currentAction !== "anxiousHop") {
      await playAnxiousHop();
    }
    return;
  }
  await playAction(ev.action);
}

async function main() {
  await listen<PetEvent>("pet-event", (e) => {
    void onPetEvent(e.payload);
  });
  await listen<AppConfigDto>("config-changed", (e) => {
    setDisplayMaxPx(e.payload?.displayMaxPx ?? DEFAULT_DISPLAY_MAX_PX);
  });
  await loadDisplayConfig();
  try {
    const status = await invoke<PetEvent>("get_status");
    if (status.characterPack) {
      await ensurePack(status.characterPack);
    }
  } catch {
    // ignore before backend ready
  }
}

void main();
