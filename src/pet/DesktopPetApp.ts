import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Dock = "none" | "left" | "right" | "top" | "bottom";

const root = document.getElementById("app")!;
root.innerHTML = `
  <div id="pet-shell" class="dock-none" title="拖动 · 双击打开主界面">
    <div class="pet-face" aria-hidden="true">
      <svg viewBox="0 0 88 88" width="88" height="88">
        <defs>
          <radialGradient id="g" cx="35%" cy="30%" r="70%">
            <stop offset="0%" stop-color="#e8f6f0"/>
            <stop offset="55%" stop-color="#7eb8a4"/>
            <stop offset="100%" stop-color="#3d7a68"/>
          </radialGradient>
        </defs>
        <circle cx="44" cy="44" r="36" fill="url(#g)" />
        <circle cx="44" cy="44" r="36" fill="none" stroke="rgba(255,255,255,0.45)" stroke-width="2"/>
        <ellipse class="eye eye-l" cx="32" cy="40" rx="5" ry="6" fill="#1d2a24"/>
        <ellipse class="eye eye-r" cx="56" cy="40" rx="5" ry="6" fill="#1d2a24"/>
        <circle cx="30" cy="38" r="1.6" fill="#fff"/>
        <circle cx="54" cy="38" r="1.6" fill="#fff"/>
        <ellipse cx="44" cy="52" rx="4" ry="3" fill="#2f6f5e"/>
      </svg>
    </div>
  </div>
`;

const shell = document.getElementById("pet-shell") as HTMLDivElement;

let dragging = false;
let startMouseX = 0;
let startMouseY = 0;
let startWinX = 0;
let startWinY = 0;

function setDock(next: Dock) {
  shell.className = `dock-${next}`;
}

async function openMain() {
  try {
    await invoke("open_main_ui", { tab: "today" });
  } catch {
    /* ignore */
  }
}

shell.addEventListener("dblclick", (e) => {
  e.preventDefault();
  void openMain();
});

shell.addEventListener("pointerdown", async (e) => {
  if (e.button !== 0) return;
  dragging = true;
  shell.setPointerCapture(e.pointerId);
  startMouseX = e.screenX;
  startMouseY = e.screenY;
  try {
    const pos = await invoke<{ x: number; y: number }>("get_pet_screen_pos");
    startWinX = pos.x;
    startWinY = pos.y;
  } catch {
    dragging = false;
  }
  setDock("none");
});

shell.addEventListener("pointermove", async (e) => {
  if (!dragging) return;
  const dx = e.screenX - startMouseX;
  const dy = e.screenY - startMouseY;
  try {
    await invoke("set_pet_position", {
      x: Math.round(startWinX + dx),
      y: Math.round(startWinY + dy),
    });
  } catch {
    /* ignore */
  }
});

async function endDrag(e: PointerEvent) {
  if (!dragging) return;
  dragging = false;
  try {
    shell.releasePointerCapture(e.pointerId);
  } catch {
    /* ignore */
  }
  try {
    const result = await invoke<{ dock: Dock }>("snap_pet");
    setDock(result.dock || "none");
  } catch {
    setDock("none");
  }
}

shell.addEventListener("pointerup", (e) => void endDrag(e));
shell.addEventListener("pointercancel", (e) => void endDrag(e));

void listen<{ visible?: boolean }>("desktop-pet-event", (e) => {
  document.body.style.opacity = e.payload?.visible === false ? "0" : "1";
});
