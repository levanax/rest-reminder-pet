import { listen } from "@tauri-apps/api/event";
import {
  isRainGlassRunning,
  onRainGlassResize,
  setRainGlassPaused,
  startRainGlass,
  stopRainGlass,
} from "./RainGlassRenderer";
import { normalizeScene, type ReminderScene, type SnowEventPayload } from "./sceneTypes";
import {
  isSnowRunning,
  onSnowResize,
  setSnowPaused,
  startSnow,
  stopSnow,
  syncSnowLayout,
} from "./SnowRenderer";

const canvas = document.getElementById("snow") as HTMLCanvasElement;

let activeScene: ReminderScene | null = null;

function stopAll() {
  stopSnow();
  stopRainGlass();
  activeScene = null;
}

function startScene(scene: ReminderScene, payload: SnowEventPayload) {
  if (scene === "rainGlass") {
    stopSnow();
    startRainGlass(canvas);
  } else {
    stopRainGlass();
    startSnow(payload);
  }
  activeScene = scene;
}

async function main() {
  await listen<{ held?: boolean }>("session-hold", (e) => {
    const held = !!e.payload?.held;
    setSnowPaused(held);
    setRainGlassPaused(held);
  });

  await listen<SnowEventPayload>("snow-event", (e) => {
    const payload = e.payload;
    if (!payload?.active) {
      stopAll();
      return;
    }
    const scene = normalizeScene(payload.scene);
    if (activeScene === scene && (isSnowRunning() || isRainGlassRunning())) {
      if (scene === "snow") syncSnowLayout(payload);
      return;
    }
    startScene(scene, payload);
  });

  window.addEventListener("resize", () => {
    onSnowResize();
    onRainGlassResize();
  });
}

void main();
