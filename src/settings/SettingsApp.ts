import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type AppConfig = {
  workMinutes: number;
  observationSeconds: number;
  snowSeconds: number;
  characterPack: string;
  autostart: boolean;
  wanxiang: { apiKey?: string | null; endpoint?: string | null };
};

const root = document.getElementById("app")!;
root.innerHTML = `
  <div class="settings">
    <h1>Rest Reminder Pet</h1>
    <p class="sub">设置工作时长与桌宠形象。托盘图标可暂停、立即提醒或退出。</p>

    <div class="field">
      <label for="workMinutes">工作时长（分钟）</label>
      <input id="workMinutes" type="number" min="1" max="180" />
    </div>

    <div class="field">
      <label for="observationSeconds">观察期（秒）</label>
      <input id="observationSeconds" type="number" min="10" max="120" />
    </div>

    <div class="field">
      <label for="snowSeconds">飘雪时长（秒）</label>
      <input id="snowSeconds" type="number" min="30" max="600" />
      <span class="sub" style="margin:0">与小猫退场无关，默认 120 秒</span>
    </div>

    <div class="field">
      <label for="characterPack">形象包</label>
      <div class="row">
        <select id="characterPack" style="flex:1"></select>
        <button type="button" class="ghost" id="importPack">导入</button>
      </div>
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
  </div>
`;

const workMinutes = document.getElementById("workMinutes") as HTMLInputElement;
const observationSeconds = document.getElementById("observationSeconds") as HTMLInputElement;
const snowSeconds = document.getElementById("snowSeconds") as HTMLInputElement;
const characterPack = document.getElementById("characterPack") as HTMLSelectElement;
const autostart = document.getElementById("autostart") as HTMLInputElement;
const apiKey = document.getElementById("apiKey") as HTMLInputElement;
const status = document.getElementById("status")!;

function setStatus(msg: string, kind: "" | "ok" | "error" = "") {
  status.textContent = msg;
  status.className = `status ${kind}`.trim();
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

async function load() {
  const cfg = await invoke<AppConfig>("get_config");
  workMinutes.value = String(cfg.workMinutes);
  observationSeconds.value = String(cfg.observationSeconds);
  snowSeconds.value = String(cfg.snowSeconds ?? 120);
  autostart.checked = !!cfg.autostart;
  apiKey.value = cfg.wanxiang?.apiKey || "";
  await refreshPacks(cfg.characterPack);
}

document.getElementById("save")!.addEventListener("click", async () => {
  try {
    const cfg: AppConfig = {
      workMinutes: Number(workMinutes.value),
      observationSeconds: Number(observationSeconds.value),
      snowSeconds: Number(snowSeconds.value),
      characterPack: characterPack.value,
      autostart: autostart.checked,
      wanxiang: {
        apiKey: apiKey.value.trim() || null,
        endpoint: null,
      },
    };
    await invoke("save_app_config", { cfg });
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

void load().catch((e) => setStatus(String(e), "error"));
