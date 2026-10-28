export type ReminderScene = "snow" | "rainGlass";

export type MonitorRectPhysical = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type SnowEventPayload = {
  active: boolean;
  scene?: ReminderScene;
  window?: { width: number; height: number };
  monitors?: MonitorRectPhysical[];
};

export function normalizeScene(raw?: string): ReminderScene {
  return raw === "rainGlass" ? "rainGlass" : "snow";
}
