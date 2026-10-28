import { listen } from "@tauri-apps/api/event";
import {
  isRainGlassRunning,
  onRainGlassResize,
  setRainGlassPaused,
  startRainGlass,
  stopRainGlass,
} from "./RainGlassRenderer";
import {
  isRainStreakRunning,
  onRainStreakResize,
  setRainStreakPaused,
  startRainStreak,
  stopRainStreak,
} from "./RainStreakRenderer";
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

function anyRunning() {
  return isSnowRunning() || isRainGlassRunning() || isRainStreakRunning();
}

function stopAll() {
  stopSnow();
  stopRainGlass();
  stopRainStreak();
  activeScene = null;
}

function startScene(scene: ReminderScene, payload: SnowEventPayload) {
  stopAll();
  if (scene === "rainGlass") {
    startRainGlass(canvas);
  } else if (scene === "rainStreak") {
    startRainStreak(canvas);
  } else {
    startSnow(payload);
  }
  activeScene = scene;
}

async function main() {
  await listen<{ held?: boolean }>("session-hold", (e) => {
    const held = !!e.payload?.held;
    setSnowPaused(held);
    setRainGlassPaused(held);
    setRainStreakPaused(held);
  });

  await listen<SnowEventPayload>("snow-event", (e) => {
    const payload = e.payload;
    if (!payload?.active) {
      stopAll();
      return;
    }
    const scene = normalizeScene(payload.scene);
    if (activeScene === scene && anyRunning()) {
      if (scene === "snow") syncSnowLayout(payload);
      return;
    }
    startScene(scene, payload);
  });

  window.addEventListener("resize", () => {
    onSnowResize();
    onRainGlassResize();
    onRainStreakResize();
  });
}

void main();
