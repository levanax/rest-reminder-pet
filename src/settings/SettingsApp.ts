import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { renderTodayPanel, type DailyRestStats } from "./TodayPanel";

type AppConfig = {
  workMinutes: number;
  observationSeconds: number;
  reminderScene?: string;
  characterPack?: string;
  displayMaxPx?: number;
  autostart: boolean;
  wanxiang: { apiKey?: string | null; endpoint?: string | null };
  snowSeconds?: number;
  desktopPetEnabled?: boolean;
  desktopPetMemoryGuard?: boolean;
  desktopPetMemoryThreshold?: number;
};

type TabId = "today" | "settings";

const root = document.getElementById("app")!;
root.innerHTML = `
  <div class="settings main-shell liquid-glass enter">
    <div class="tabs" role="tablist">
      <button type="button" class="tab" role="tab" data-tab="today" id="tab-today">今日</button>
      <button type="button" class="tab" role="tab" data-tab="settings" id="tab-settings">设置</button>
    </div>

    <section class="panel" id="panel-today" role="tabpanel"></section>

    <section class="panel" id="panel-settings" role="tabpanel" hidden>
      <p class="sub">工作时长、提醒景致与桌宠。关闭本窗口会销毁界面以节省内存，可从托盘再次打开。</p>

      <div class="field">
        <label for="workMinutes">工作时长（分钟）</label>
        <input id="workMinutes" type="number" min="1" max="180" />
      </div>

      <div class="field">
        <label for="observationSeconds">连续无键鼠才算休息成功（秒）</label>
        <input id="observationSeconds" type="number" min="10" max="120" />
        <span class="sub" style="margin:0">有操作会重新计时；未达标前景致一直持续</span>
      </div>

      <div class="field">
        <label for="reminderScene">提醒景致</label>
        <select id="reminderScene">
          <option value="random">随机</option>
          <option value="snow">下雪</option>
          <option value="rainGlass">玻璃起雾</option>
          <option value="rainStreak">玻璃下雨</option>
          <option value="blinds">百叶窗</option>
          <option value="waterFlow">水帘</option>
        </select>
        <span class="sub" style="margin:0">默认随机；每次提醒开始时决定，中途不切换</span>
      </div>

      <div class="field glass-card">
        <label class="row" for="desktopPetEnabled">
          <input id="desktopPetEnabled" type="checkbox" />
          显示桌面宠物
        </label>
        <span class="sub" style="margin:0">可拖动，靠近边缘会贴边偷看</span>
      </div>

      <div class="field glass-card">
        <label class="row" for="desktopPetMemoryGuard">
          <input id="desktopPetMemoryGuard" type="checkbox" />
          内存过高时自动关闭桌宠
        </label>
        <label for="desktopPetMemoryThreshold">内存阈值（%）</label>
        <input id="desktopPetMemoryThreshold" type="number" min="50" max="99" />
        <span class="sub" style="margin:0">默认 85；关闭后不会自动再开，需手动开启</span>
      </div>

      <div class="field">
        <label class="row" for="autostart">
          <input id="autostart" type="checkbox" />
          登录 Windows 时启动（默认关闭）
        </label>
      </div>

      <div class="field">
        <label for="apiKey">阿里云万相 API Key</label>
        <input id="apiKey" type="password" autocomplete="off" placeholder="仅保存在本机" />
      </div>

      <div class="actions">
        <button type="button" class="primary" id="save">保存</button>
        <button type="button" class="ghost" id="remind">立即提醒</button>
      </div>
      <div class="status" id="status"></div>
    </section>
  </div>
`;

const panelToday = document.getElementById("panel-today")!;
const panelSettings = document.getElementById("panel-settings")!;
const workMinutes = document.getElementById("workMinutes") as HTMLInputElement;
const observationSeconds = document.getElementById("observationSeconds") as HTMLInputElement;
const reminderScene = document.getElementById("reminderScene") as HTMLSelectElement;
const desktopPetEnabled = document.getElementById("desktopPetEnabled") as HTMLInputElement;
const desktopPetMemoryGuard = document.getElementById(
  "desktopPetMemoryGuard",
) as HTMLInputElement;
const desktopPetMemoryThreshold = document.getElementById(
  "desktopPetMemoryThreshold",
) as HTMLInputElement;
const autostart = document.getElementById("autostart") as HTMLInputElement;
const apiKey = document.getElementById("apiKey") as HTMLInputElement;
const status = document.getElementById("status")!;
const shell = root.querySelector(".liquid-glass") as HTMLElement;

let currentTab: TabId = "today";
let cachedCfg: AppConfig | null = null;

function setStatus(msg: string, kind: "" | "ok" | "error" = "") {
  status.textContent = msg;
  status.className = `status ${kind}`.trim();
}

function setTab(tab: TabId) {
  currentTab = tab;
  const isToday = tab === "today";
  panelToday.hidden = !isToday;
  panelSettings.hidden = isToday;
  document.getElementById("tab-today")!.classList.toggle("active", isToday);
  document.getElementById("tab-settings")!.classList.toggle("active", !isToday);
  if (isToday) void refreshToday();
}

async function refreshToday() {
  try {
    const stats = await invoke<DailyRestStats>("get_daily_stats");
    const wm = cachedCfg?.workMinutes ?? (Number(workMinutes.value) || 20);
    renderTodayPanel(panelToday, stats, wm);
  } catch (e) {
    panelToday.innerHTML = `<p class="empty">无法加载统计：${String(e)}</p>`;
  }
}

async function load() {
  const cfg = await invoke<AppConfig>("get_config");
  cachedCfg = cfg;
  workMinutes.value = String(cfg.workMinutes);
  observationSeconds.value = String(cfg.observationSeconds);
  reminderScene.value = cfg.reminderScene || "random";
  desktopPetEnabled.checked = cfg.desktopPetEnabled !== false;
  desktopPetMemoryGuard.checked = cfg.desktopPetMemoryGuard !== false;
  desktopPetMemoryThreshold.value = String(cfg.desktopPetMemoryThreshold ?? 85);
  autostart.checked = !!cfg.autostart;
  apiKey.value = cfg.wanxiang?.apiKey || "";
  setTab(currentTab);
  requestAnimationFrame(() => shell.classList.add("enter-active"));
}

document.getElementById("tab-today")!.addEventListener("click", () => setTab("today"));
document.getElementById("tab-settings")!.addEventListener("click", () => setTab("settings"));

document.getElementById("save")!.addEventListener("click", async () => {
  try {
    const cfg: AppConfig = {
      workMinutes: Number(workMinutes.value),
      observationSeconds: Number(observationSeconds.value),
      reminderScene: reminderScene.value || "random",
      characterPack: cachedCfg?.characterPack || "default",
      displayMaxPx: cachedCfg?.displayMaxPx ?? 96,
      autostart: autostart.checked,
      desktopPetEnabled: desktopPetEnabled.checked,
      desktopPetMemoryGuard: desktopPetMemoryGuard.checked,
      desktopPetMemoryThreshold: Number(desktopPetMemoryThreshold.value) || 85,
      wanxiang: {
        apiKey: apiKey.value.trim() || null,
        endpoint: null,
      },
    };
    await invoke("save_app_config", { cfg });
    cachedCfg = cfg;
    setStatus("已保存", "ok");
  } catch (e) {
    setStatus(String(e), "error");
  }
});

document.getElementById("remind")!.addEventListener("click", async () => {
  try {
    await invoke("remind_now");
    setStatus("已触发提醒", "ok");
  } catch (e) {
    setStatus(String(e), "error");
  }
});

void listen<{ tab?: string }>("main-ui-open", (e) => {
  shell.classList.remove("enter-active");
  void shell.offsetWidth;
  shell.classList.add("enter");
  requestAnimationFrame(() => shell.classList.add("enter-active"));
  const tab = e.payload?.tab === "settings" ? "settings" : "today";
  setTab(tab);
  if (tab === "today") void refreshToday();
});

void listen<{ memoryPercent?: number }>("desktop-pet-memory-closed", (e) => {
  desktopPetEnabled.checked = false;
  const pct = e.payload?.memoryPercent;
  setStatus(
    pct != null ? `系统内存约 ${Math.round(pct)}%，桌宠已关闭` : "内存过高，桌宠已关闭",
    "error",
  );
  setTab("settings");
});

void load().catch((e) => setStatus(String(e), "error"));
