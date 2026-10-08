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

const root = document.getElementById("app")!;
root.innerHTML = `
  <div id="stage" style="position:relative;width:100%;height:100%;background:transparent;">
    <img id="pet" alt="" draggable="false"
      style="position:absolute;left:50%;top:-160px;width:128px;height:128px;transform:translateX(-50%);
             image-rendering:auto;pointer-events:none;opacity:0;" />
  </div>
`;

const img = document.getElementById("pet") as HTMLImageElement;
let pack: Pack | null = null;
let loadedPackId: string | null = null;
let animTimer: number | null = null;
let moveRaf: number | null = null;
let currentAction: string | null = null;

async function ensurePack(name: string) {
  if (pack && loadedPackId === name) return pack;
  pack = await invoke<Pack>("get_pack", { name });
  loadedPackId = name;
  return pack;
}

function clearTimers() {
  if (animTimer != null) {
    window.clearInterval(animTimer);
    animTimer = null;
  }
  if (moveRaf != null) {
    window.cancelAnimationFrame(moveRaf);
    moveRaf = null;
  }
}

function setY(pct: number) {
  const h = window.innerHeight;
  const y = Math.max(0, Math.min(h - 128, (h * pct) / 100));
  img.style.top = `${y}px`;
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
  const def = pack.manifest.actions[action];
  if (!def) {
    await invoke("notify_action_done", { action });
    return;
  }
  clearTimers();
  currentAction = action;
  img.style.opacity = "1";

  let frame = 0;
  const applyFrame = () => {
    const file = def.frames[frame % def.frames.length];
    img.src = pack!.frames[file] || "";
    frame++;
  };
  applyFrame();
  animTimer = window.setInterval(applyFrame, Math.max(40, 1000 / (def.fps || 8)));

  const durationMs = Math.max(800, (def.frames.length / Math.max(1, def.fps)) * 1000 * (action === "lookDown" ? 1.2 : 1.6));

  if (action === "crawl") {
    img.style.top = "-140px";
    animateVertical(-140, 8, durationMs, async () => {
      clearTimers();
      await invoke("notify_action_done", { action: "crawl" });
    });
  } else if (action === "lookDown") {
    setY(6);
    window.setTimeout(async () => {
      clearTimers();
      await invoke("notify_action_done", { action: "lookDown" });
    }, Math.max(900, durationMs));
  } else if (action === "happyClimb") {
    const start = parseFloat(img.style.top) || window.innerHeight * 0.06;
    animateVertical(start, -160, durationMs, async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "happyClimb" });
    });
  } else if (action === "fall") {
    const start = parseFloat(img.style.top) || window.innerHeight * 0.06;
    animateVertical(start, window.innerHeight - 140, durationMs, async () => {
      img.style.opacity = "0";
      clearTimers();
      await invoke("notify_action_done", { action: "fall" });
    });
  }
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
  clearTimers();
  currentAction = "jumpUp";
  img.style.opacity = "1";
  let frame = 0;
  animTimer = window.setInterval(() => {
    const file = def.frames[frame % def.frames.length];
    img.src = pack!.frames[file] || "";
    frame++;
  }, Math.max(50, 1000 / (def.fps || 10)));

  const from = window.innerHeight - 20;
  const to = -170;
  img.style.top = `${from}px`;
  // 从底部弹起：先快后慢，略带回弹感
  animateJump(from, to, 1150, async () => {
    img.style.opacity = "0";
    clearTimers();
    await invoke("notify_action_done", { action: "jumpUp" });
  });
}

function animateJump(from: number, to: number, ms: number, done: () => void) {
  const start = performance.now();
  const tick = (now: number) => {
    const t = Math.min(1, (now - start) / ms);
    // easeOutBack：冲上去并稍微越过再回落一点（视觉上像跳）
    const c1 = 1.70158;
    const c3 = c1 + 1;
    const eased = 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
    img.style.top = `${from + (to - from) * Math.min(1.08, Math.max(0, eased))}px`;
    if (t < 1) {
      moveRaf = window.requestAnimationFrame(tick);
    } else {
      moveRaf = null;
      done();
    }
  };
  moveRaf = window.requestAnimationFrame(tick);
}

async function playSneakPeek() {
  if (!pack) return;
  const def = pack.manifest.actions.sneakPeek ?? pack.manifest.actions.lookDown;
  if (!def?.frames?.length) {
    await invoke("notify_action_done", { action: "sneakPeek" });
    return;
  }
  clearTimers();
  currentAction = "sneakPeek";
  img.style.opacity = "1";
  let frame = 0;
  animTimer = window.setInterval(() => {
    const file = def.frames[frame % def.frames.length];
    img.src = pack!.frames[file] || "";
    frame++;
  }, Math.max(60, 1000 / (def.fps || 4)));
  // 只探出头顶一点，停一下再缩回去
  img.style.top = "-110px";
  animateVertical(-110, -36, 700, () => {
    window.setTimeout(() => {
      animateVertical(-36, -130, 650, async () => {
        img.style.opacity = "0";
        clearTimers();
        await invoke("notify_action_done", { action: "sneakPeek" });
      });
    }, 1100);
  });
}

function animateVertical(from: number, to: number, ms: number, done: () => void) {
  const start = performance.now();
  const tick = (now: number) => {
    const t = Math.min(1, (now - start) / ms);
    const eased = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
    img.style.top = `${from + (to - from) * eased}px`;
    if (t < 1) {
      moveRaf = window.requestAnimationFrame(tick);
    } else {
      moveRaf = null;
      done();
    }
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
    // keep looping lookDown while observing
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

async function playLookLoop() {
  if (!pack) return;
  const def = pack.manifest.actions.lookDown;
  if (!def) return;
  clearTimers();
  currentAction = "lookDown";
  setY(6);
  img.style.opacity = "1";
  let frame = 0;
  animTimer = window.setInterval(() => {
    const file = def.frames[frame % def.frames.length];
    img.src = pack!.frames[file] || "";
    frame++;
  }, Math.max(80, 1000 / (def.fps || 4)));
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
