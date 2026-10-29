import { listen } from "@tauri-apps/api/event";
import {
  isBlindsRunning,
  onBlindsResize,
  setBlindsPaused,
  startBlinds,
  stopBlinds,
} from "./BlindsRenderer";
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
  return isSnowRunning() || isRainGlassRunning() || isRainStreakRunning() || isBlindsRunning();
}

function stopAll() {
  stopSnow();
  stopRainGlass();
  stopRainStreak();
  stopBlinds();
  activeScene = null;
}

function startScene(scene: ReminderScene, payload: SnowEventPayload) {
  stopAll();
  if (scene === "rainGlass") {
    startRainGlass(canvas);
  } else if (scene === "rainStreak") {
    startRainStreak(canvas);
  } else if (scene === "blinds") {
    startBlinds(canvas);
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
    setBlindsPaused(held);
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
    onBlindsResize();
  });
}

void main();
