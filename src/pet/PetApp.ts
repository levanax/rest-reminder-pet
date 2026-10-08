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

const root = document.getElementById("app")!;
root.innerHTML = `
  <div id="stage" style="position:relative;width:100%;height:100%;background:transparent;">
    <img id="pet" alt="" draggable="false"
      style="position:absolute;left:50%;top:-160px;width:128px;height:128px;transform:translateX(-50%);
             image-rendering:auto;pointer-events:none;opacity:0;transform-origin:50% 40%;" />
  </div>
`;

const img = document.getElementById("pet") as HTMLImageElement;
let pack: Pack | null = null;
let loadedPackId: string | null = null;
let moveRaf: number | null = null;
let currentAction: string | null = null;
let lastFrameFile: string | null = null;

async function ensurePack(name: string) {
  if (pack && loadedPackId === name) return pack;
  pack = await invoke<Pack>("get_pack", { name });
  loadedPackId = name;
  return pack;
}

function clearTimers() {
  if (moveRaf != null) {
    window.cancelAnimationFrame(moveRaf);
    moveRaf = null;
  }
}

function yFromPct(pct: number) {
  const h = window.innerHeight;
  return Math.max(0, Math.min(h - 128, (h * pct) / 100));
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
  // Remap time so velocity drops near each pause center
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
      return ((i - 1 + local) / samples);
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
  const from = window.innerHeight - 20;
  const to = -170;
  const crouchY = from + 8;

  runTimeline(
    1300,
    def.frames,
    (t) => {
      let y: number;
      let sx = 1;
      let sy = 1;
      let opacity = 1;
      let frameT = t;

      if (t < 0.15) {
        const u = t / 0.15;
        y = lerp(from, crouchY, u);
        sx = lerp(1, 1.1, u);
        sy = lerp(1, 0.88, u);
        frameT = u * 0.16;
      } else if (t < 0.55) {
        const u = (t - 0.15) / 0.4;
        const e = easeOutCubic(u);
        y = lerp(crouchY, lerp(from, to, 0.72), e);
        sx = lerp(1.1, 0.92, u);
        sy = lerp(0.88, 1.12, u);
        frameT = 0.16 + u * 0.4;
      } else if (t < 0.85) {
        const u = (t - 0.55) / 0.3;
        y = lerp(lerp(from, to, 0.72), to, easeOutCubic(u));
        sx = lerp(0.92, 1.02, u);
        sy = lerp(1.12, 0.97, u);
        frameT = 0.56 + u * 0.28;
      } else {
        const u = (t - 0.85) / 0.15;
        y = to;
        sx = lerp(1.02, 1, u);
        sy = lerp(0.97, 1.04, Math.sin(u * Math.PI));
        opacity = lerp(1, 0, u);
        frameT = 0.84 + u * 0.16;
      }

      return {
        y,
        x: 0,
        rot: t < 0.15 ? 0 : (t < 0.55 ? lerp(0, -4, (t - 0.15) / 0.4) : lerp(-4, 0, (t - 0.55) / 0.45)),
        sx,
        sy,
        opacity,
        frameT: Math.min(1, frameT),
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
  const from = -150;
  const to = 8;

  runTimeline(
    1400,
    frames,
    (t) => {
      const p = smoothstep(pausedProgress(t, [0.28, 0.52, 0.76], 0.07));
      const sway = Math.sin(p * Math.PI * 3);
      return {
        y: lerp(from, to, p),
        x: sway * 10,
        rot: sway * 8,
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
  const y = yFromPct(6);

  runTimeline(
    1000,
    frames,
    (t) => {
      const sway = Math.sin(t * Math.PI * 2) * (1 - t);
      return {
        y,
        x: sway * 3,
        rot: sway * 2,
        sx: 1,
        sy: 1,
        opacity: 1,
        frameT: Math.min(0.8, t), // avoid forced blink on one-shot
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
  const from = parseFloat(img.style.top) || yFromPct(6);
  const to = -170;

  runTimeline(
    1200,
    frames,
    (t) => {
      const p = easeInOutQuad(pausedProgress(t, [0.35, 0.65], 0.07));
      const sway = Math.sin(p * Math.PI * 2.5);
      const fade = t > 0.75 ? (t - 0.75) / 0.25 : 0;
      return {
        y: lerp(from, to, p),
        x: sway * 9,
        rot: sway * 9,
        sx: 1,
        sy: 1 + Math.abs(sway) * 0.04,
        opacity: 1 - fade,
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
  const from = parseFloat(img.style.top) || yFromPct(6);
  const to = window.innerHeight - 120;

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
  const def = pack.manifest.actions.sneakPeek ?? pack.manifest.actions.lookDown;
  if (!def?.frames?.length) {
    await invoke("notify_action_done", { action: "sneakPeek" });
    return;
  }
  currentAction = "sneakPeek";
  const from = -110;
  const peek = -28;
  const hide = -130;
  const total = 2800;
  const outEnd = 900 / total;
  const holdEnd = (900 + 1200) / total;

  runTimeline(
    total,
    def.frames,
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
        sy: 1 + (micro ? breath * 0.025 : 0),
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

async function playLookLoop() {
  if (!pack) return;
  const frames = actionFrames("lookDown");
  if (!frames.length) return;
  clearTimers();
  currentAction = "lookDown";
  lastFrameFile = null;
  const y = yFromPct(6);
  let nextBlinkAt = performance.now() + 2800 + Math.random() * 1400;
  let blinkUntil = 0;

  const tick = (now: number) => {
    if (currentAction !== "lookDown") return;

    if (now >= nextBlinkAt) {
      blinkUntil = now + 120;
      nextBlinkAt = now + 2800 + Math.random() * 1400;
    }

    const breath = Math.sin(now / 900);
    const scan = Math.sin(now / 1800);
    const blinking = now < blinkUntil;
    // blink frame is last of 5 → frameT near 0.9+
    const frameT = blinking
      ? 0.92
      : 0.15 + (0.5 + 0.5 * Math.sin(now / 2200)) * 0.55;

    applyPose(
      {
        y,
        x: scan * 4,
        rot: scan * 3,
        sx: 1 - breath * 0.012,
        sy: 1 + breath * 0.025,
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
  if (ev.phase === "observing" && ev.action === "lookDown") {
    if (currentAction !== "lookDown") {
      await playLookLoop();
    }
    return;
  }
  await playAction(ev.action);
}

async function main() {
  await listen<PetEvent>("pet-event", (e) => {
    void onPetEvent(e.payload);
  });
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
