import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type SummaryEvent = {
  successCount: number;
  reminderCount: number;
  thisDelaySecs: number;
  delayTotalSecs: number;
};

function formatDelay(secs: number): string {
  const total = Math.max(0, Math.floor(secs));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) return `${hours}小时${minutes}分${seconds}秒`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}

const root = document.getElementById("app");
if (!root) {
  throw new Error("missing #app");
}

const shell = document.createElement("button");
shell.type = "button";
shell.id = "summary";
shell.innerHTML = `
  <p class="kicker">今日休息</p>
  <p class="count" data-count>0 / 0</p>
  <p class="line" data-this>本次拖延 0秒</p>
  <p class="line" data-total>今日累计拖延 0秒</p>
  <p class="hint">点击关闭</p>
`;
root.appendChild(shell);

const style = document.createElement("style");
style.textContent = `
  #summary {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 14px 16px 12px;
    border: 1px solid #d5e0da;
    border-radius: 12px;
    text-align: left;
    cursor: pointer;
    color: #1d2a24;
    background:
      radial-gradient(circle at 100% 0%, #dff3ea 0%, transparent 42%),
      linear-gradient(165deg, #f7faf8, #e7f0eb);
    font-family: "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  #summary .kicker {
    margin: 0;
    font-size: 12px;
    letter-spacing: 0.04em;
    color: #5c6f66;
  }
  #summary .count {
    margin: 2px 0 8px;
    font-size: 28px;
    font-weight: 650;
    line-height: 1.1;
  }
  #summary .line {
    margin: 0 0 4px;
    font-size: 13px;
  }
  #summary .hint {
    margin: 8px 0 0;
    font-size: 11px;
    color: #5c6f66;
  }
`;
document.head.appendChild(style);

const countEl = shell.querySelector("[data-count]");
const thisEl = shell.querySelector("[data-this]");
const totalEl = shell.querySelector("[data-total]");

let hideTimer = 0;
let hideRemaining = 0;
let hideDeadline = 0;
let summaryOpen = false;

function render(payload: SummaryEvent) {
  if (countEl) countEl.textContent = `${payload.successCount} / ${payload.reminderCount}`;
  if (thisEl) thisEl.textContent = `本次拖延 ${formatDelay(payload.thisDelaySecs)}`;
  if (totalEl) totalEl.textContent = `今日累计拖延 ${formatDelay(payload.delayTotalSecs)}`;
}

function armHide(ms: number) {
  window.clearTimeout(hideTimer);
  hideRemaining = ms;
  hideDeadline = performance.now() + ms;
  hideTimer = window.setTimeout(() => {
    void closeSummary();
  }, ms);
}

async function closeSummary() {
  summaryOpen = false;
  window.clearTimeout(hideTimer);
  hideRemaining = 0;
  await invoke("hide_summary_cmd");
}

shell.addEventListener("click", () => {
  void closeSummary();
});

void listen<SummaryEvent>("summary-event", (event) => {
  render(event.payload);
  summaryOpen = true;
  armHide(5000);
});

void listen<{ held?: boolean }>("session-hold", (event) => {
  if (!summaryOpen) return;
  if (event.payload?.held) {
    hideRemaining = Math.max(0, hideDeadline - performance.now());
    window.clearTimeout(hideTimer);
    return;
  }
  if (hideRemaining > 0) armHide(hideRemaining);
});
