export type RestOutcome = "success" | "incomplete";

export type RestEvent = {
  at: string;
  outcome: RestOutcome;
  delaySecs: number;
};

export type DailyRestStats = {
  date: string;
  successCount: number;
  incompleteCount: number;
  delayTotalSecs: number;
  events?: RestEvent[];
};

function formatDelay(secs: number): string {
  const total = Math.max(0, Math.floor(secs));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  if (hours > 0) return `${hours}小时${minutes}分`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}

function parseEventMinutes(at: string): number | null {
  // 2026-10-28T14:32:05 or with space
  const m = at.match(/T(\d{2}):(\d{2})/) || at.match(/\s(\d{2}):(\d{2})/);
  if (!m) return null;
  return Number(m[1]) * 60 + Number(m[2]);
}

function expectedRests(workMinutes: number): number {
  const now = new Date();
  const dayStart = new Date(now);
  dayStart.setHours(8, 0, 0, 0);
  const dayEnd = new Date(now);
  dayEnd.setHours(22, 0, 0, 0);
  const from = Math.max(dayStart.getTime(), Math.min(now.getTime(), dayEnd.getTime()));
  const elapsedMin = Math.max(0, (from - dayStart.getTime()) / 60000);
  const wm = Math.max(1, workMinutes);
  return Math.max(0, Math.floor(elapsedMin / wm));
}

function axisRange(events: RestEvent[]): { startMin: number; endMin: number } {
  const mins = events
    .map((e) => parseEventMinutes(e.at))
    .filter((n): n is number => n != null);
  if (!mins.length) return { startMin: 8 * 60, endMin: 22 * 60 };
  const lo = Math.min(...mins);
  const hi = Math.max(...mins);
  const startMin = Math.max(0, Math.floor((lo - 60) / 60) * 60);
  const endMin = Math.min(24 * 60, Math.ceil((hi + 60) / 60) * 60);
  if (endMin - startMin < 4 * 60) {
    return { startMin: Math.max(0, startMin - 60), endMin: Math.min(24 * 60, endMin + 60) };
  }
  return { startMin, endMin };
}

function fmtAxis(min: number): string {
  const h = Math.floor(min / 60);
  const m = min % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
}

export function renderTodayPanel(
  root: HTMLElement,
  stats: DailyRestStats,
  workMinutes: number,
): void {
  const events = stats.events ?? [];
  const reminderCount = stats.successCount + stats.incompleteCount;
  const expected = expectedRests(workMinutes);
  const { startMin, endMin } = axisRange(events);
  const span = Math.max(1, endMin - startMin);
  const maxDelay = Math.max(1, ...events.map((e) => e.delaySecs), 1);
  const chartH = 120;
  const padL = 8;
  const padR = 8;
  const padT = 12;
  const padB = 28;
  const plotW = 440;
  const plotH = chartH - padT - padB;

  const ticks = [startMin, Math.round((startMin + endMin) / 2), endMin];

  const lollipops = events
    .map((e) => {
      const min = parseEventMinutes(e.at);
      if (min == null) return null;
      const x = padL + ((min - startMin) / span) * (plotW - padL - padR);
      const h = Math.max(8, (e.delaySecs / maxDelay) * plotH);
      const y = padT + plotH - h;
      const color = e.outcome === "success" ? "#2f6f5e" : "#c47a3a";
      return { x, y, h, color, e };
    })
    .filter(Boolean) as { x: number; y: number; h: number; color: string; e: RestEvent }[];

  const listHtml =
    events.length === 0
      ? `<p class="empty">今天还没有休息记录</p>`
      : `<ul class="event-list">${events
          .slice()
          .reverse()
          .map((e) => {
            const time = e.at.includes("T") ? e.at.split("T")[1]?.slice(0, 5) : e.at.slice(11, 16);
            const label = e.outcome === "success" ? "成功" : "未完成";
            const tone = e.outcome === "success" ? "ok" : "warn";
            const pct = Math.min(100, Math.round((e.delaySecs / maxDelay) * 100));
            return `<li class="${tone}">
              <span class="t">${time || "--:--"}</span>
              <span class="o">${label}</span>
              <span class="d">拖延 ${formatDelay(e.delaySecs)}</span>
              <span class="bar"><i style="width:${pct}%"></i></span>
            </li>`;
          })
          .join("")}</ul>`;

  root.innerHTML = `
    <div class="kpi-row">
      <div class="kpi"><span class="k">成功</span><span class="v">${stats.successCount}</span></div>
      <div class="kpi"><span class="k">提醒</span><span class="v">${reminderCount}</span></div>
      <div class="kpi"><span class="k">累计拖延</span><span class="v">${formatDelay(stats.delayTotalSecs)}</span></div>
    </div>
    <p class="hint-line">按当前工作时长，今天大约该休息 ${expected} 次（仅提示）</p>
    <h2 class="section-title">今日休息</h2>
    <svg class="lollipop" viewBox="0 0 ${plotW} ${chartH}" role="img" aria-label="今日休息时间轴">
      <line x1="${padL}" y1="${padT + plotH}" x2="${plotW - padR}" y2="${padT + plotH}" stroke="#d5e0da" stroke-width="1.5"/>
      ${ticks
        .map((t) => {
          const x = padL + ((t - startMin) / span) * (plotW - padL - padR);
          return `<text x="${x}" y="${chartH - 8}" text-anchor="middle" fill="#5c6f66" font-size="11">${fmtAxis(t)}</text>`;
        })
        .join("")}
      ${lollipops
        .map(
          (p) => `
        <line x1="${p.x}" y1="${padT + plotH}" x2="${p.x}" y2="${p.y}" stroke="${p.color}" stroke-width="2.5" stroke-linecap="round"/>
        <circle cx="${p.x}" cy="${p.y}" r="6" fill="${p.color}"/>
      `,
        )
        .join("")}
      ${
        lollipops.length === 0
          ? `<text x="${plotW / 2}" y="${padT + plotH / 2}" text-anchor="middle" fill="#5c6f66" font-size="12">暂无数据</text>`
          : ""
      }
    </svg>
    <div class="legend">
      <span><i class="dot ok"></i>成功</span>
      <span><i class="dot warn"></i>未完成</span>
      <span class="muted">杆越高 = 拖延越久</span>
    </div>
    <h2 class="section-title">记录</h2>
    ${listHtml}
  `;
}
