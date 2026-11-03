export type ReminderScene =
  | "snow"
  | "rainGlass"
  | "rainStreak"
  | "blinds"
  | "waterFlow";

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
  if (raw === "rainGlass") return "rainGlass";
  if (raw === "rainStreak") return "rainStreak";
  if (raw === "blinds") return "blinds";
  if (raw === "waterFlow") return "waterFlow";
  return "snow";
}
