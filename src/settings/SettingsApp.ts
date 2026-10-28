import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { renderTodayPanel, type DailyRestStats } from "./TodayPanel";

type AppConfig = {
  workMinutes: number;
  observationSeconds: number;
  reminderScene?: string;
  characterPack: string;
  displayMaxPx: number;
  autostart: boolean;
  wanxiang: { apiKey?: string | null; endpoint?: string | null };
  snowSeconds?: number;
};

type TabId = "today" | "settings";

const root = document.getElementById("app")!;
root.innerHTML = `
  <div class="settings main-shell">
    <h1>Rest Reminder Pet</h1>
    <div class="tabs" role="tablist">
      <button type="button" class="tab" role="tab" data-tab="today" id="tab-today">今日</button>
      <button type="button" class="tab" role="tab" data-tab="settings" id="tab-settings">设置</button>
    </div>

    <section class="panel" id="panel-today" role="tabpanel"></section>

    <section class="panel" id="panel-settings" role="tabpanel" hidden>
      <p class="sub">设置工作时长、提醒景致与桌宠形象。托盘可暂停、立即提醒或退出。</p>

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
          <option value="rainGlass">雨雾玻璃</option>
        </select>
        <span class="sub" style="margin:0">默认随机；每次提醒开始时决定，中途不切换</span>
      </div>

      <div class="field">
        <label for="characterPack">形象包</label>
        <div class="row">
          <select id="characterPack" style="flex:1"></select>
          <button type="button" class="ghost" id="importPack">导入</button>
        </div>
      </div>

      <div class="field">
        <label for="displayMaxPx">猫咪显示大小（像素）</label>
        <input id="displayMaxPx" type="number" min="48" max="192" step="8" />
        <span class="sub" style="margin:0">资源分辨率不变，仅缩放屏上显示；默认 96，范围 48–192</span>
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
const characterPack = document.getElementById("characterPack") as HTMLSelectElement;
const displayMaxPx = document.getElementById("displayMaxPx") as HTMLInputElement;
const autostart = document.getElementById("autostart") as HTMLInputElement;
const apiKey = document.getElementById("apiKey") as HTMLInputElement;
const status = document.getElementById("status")!;

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

async function refreshPacks(selected?: string) {
  const packs = await invoke<string[]>("list_packs");
  characterPack.innerHTML = "";
  for (const name of packs) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    characterPack.appendChild(opt);
  }
  if (selected && packs.includes(selected)) {
    characterPack.value = selected;
  }
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
  displayMaxPx.value = String(cfg.displayMaxPx ?? 96);
  autostart.checked = !!cfg.autostart;
  apiKey.value = cfg.wanxiang?.apiKey || "";
  await refreshPacks(cfg.characterPack);
  setTab(currentTab);
}

document.getElementById("tab-today")!.addEventListener("click", () => setTab("today"));
document.getElementById("tab-settings")!.addEventListener("click", () => setTab("settings"));

document.getElementById("save")!.addEventListener("click", async () => {
  try {
    const cfg: AppConfig = {
      workMinutes: Number(workMinutes.value),
      observationSeconds: Number(observationSeconds.value),
      reminderScene: reminderScene.value || "random",
      characterPack: characterPack.value,
      displayMaxPx: Number(displayMaxPx.value) || 96,
      autostart: autostart.checked,
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

document.getElementById("importPack")!.addEventListener("click", async () => {
  try {
    const selected = await open({ directory: true, multiple: false });
    if (!selected || Array.isArray(selected)) return;
    const name = await invoke<string>("import_pack", { path: selected });
    await refreshPacks(name);
    setStatus(`已导入形象包：${name}`, "ok");
  } catch (e) {
    setStatus(String(e), "error");
  }
});

void listen<{ tab?: string }>("main-ui-open", (e) => {
  const tab = e.payload?.tab === "settings" ? "settings" : "today";
  setTab(tab);
  if (tab === "today") void refreshToday();
});

void load().catch((e) => setStatus(String(e), "error"));
