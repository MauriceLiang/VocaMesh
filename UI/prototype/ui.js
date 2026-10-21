/* VocaMesh prototype · icons + small render helpers */

(function () {
  const P = {
    dashboard: "M4 4h6v7H4zM14 4h6v4h-6zM14 12h6v8h-6zM4 15h6v5H4z",
    list: "M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01",
    shield: "M12 3l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6zM9 12l2 2 4-4",
    history: "M3.5 12a8.5 8.5 0 1 0 2.8-6.3M3 4v5h5M12 8v4l3 2",
    bot: "M7 8h10a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM12 4v4M9.5 13.5h.01M14.5 13.5h.01",
    terminal: "M5 5h14v14H5zM8 10l2 2-2 2M13 14h3",
    branch: "M7 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM7 8v9M17 4a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM17 8v3a4 4 0 0 1-4 4H9",
    folder: "M4 7h6l2 2h8v9H4z",
    bell: "M6 9a6 6 0 1 1 12 0c0 4 2 5 2 5H4s2-1 2-5M10 19a2 2 0 0 0 4 0",
    settings: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 7 7M17 17l1.4 1.4M18.4 5.6 17 7M7 17l-1.4 1.4",
    help: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.5 9.2A2.6 2.6 0 1 1 13 11.6c-.7.4-1 1-1 1.8M12 17h.01",
    check: "m5 12.5 4.5 4.5L19 7",
    x: "M6 6l12 12M18 6 6 18",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM16.5 16.5 21 21",
    alert: "M12 4 2.5 20h19zM12 10v4M12 17h.01",
    pause: "M9 5v14M15 5v14",
    play: "M8 5l11 7-11 7z",
    square: "M6 6h12v12H6z",
    refresh: "M20 11a8 8 0 1 0-2 6M20 5v6h-6",
    copy: "M9 9h10v10H9zM5 15V5h10",
    clip: "M9.5 13.5 16 7a3 3 0 0 1 4 4l-7.5 7.5a4.5 4.5 0 0 1-6.5-6.5l7-7",
    send: "M12 20V5M6 11l6-6 6 6",
    sun: "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zM12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.5 1.5M17.5 17.5 19 19M19 5l-1.5 1.5M6.5 17.5 5 19",
    moon: "M20 14A8 8 0 0 1 10 4a9 9 0 1 0 10 10z",
    checkcheck: "m2 13 4 4 8-9M12 15l2 2 8-9",
    more: "M6 12h.01M12 12h.01M18 12h.01",
    lock: "M7 11h10v8H7zM9 11V8a3 3 0 0 1 6 0v3",
    file: "M7 3h7l4 4v14H7zM14 3v4h4",
    external: "M14 5h5v5M19 5l-8 8M12 5H6v14h6",
    info: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01",
    bulb: "M9.5 18h5M10.5 21h3M12 3a5 5 0 0 0-3 9c.6.5 1 1.4 1 2h4c0-.6.4-1.5 1-2a5 5 0 0 0-3-9z",
    plug: "M9 3v6M15 3v6M7 9h10v3a5 5 0 0 1-10 0zM12 17v4",
    loader: "M12 3a9 9 0 1 0 9 9",
    plus: "M12 5v14M5 12h14",
    down: "m6 9 6 6 6-6",
    right: "m9 6 6 6-6 6",
    left: "m15 6-6 6 6 6",
    up: "M12 20V5M6 11l6-6 6 6",
    upright: "M7 17 17 7M9 7h8v8",
    users: "M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM2 20c0-3 2.7-5 6-5s6 2 6 5M16 8a3 3 0 1 1 0 6M22 19.5c0-2-1.3-3.6-3.4-4.2",
    panel: "M4 5h16v14H4zM9 5v14",
    sliders: "M4 7h9M17 7h3M4 12h3M11 12h9M4 17h11M19 17h1M15 5v4M9 10v4M17 15v4",
    clock: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2",
    dot: "M12 12h.01",
    zap: "M13 3 5 14h6l-1 7 8-11h-6z",
    workflow: "M5 5h5v5H5zM14 14h5v5h-5zM10 7.5h4a2 2 0 0 1 2 2v4",
    eye: "M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    pencil: "M4 20h4L20 8l-4-4L4 16z",
    shieldcheck: "M12 3l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9V6zM9 12l2 2 4-4",
    hourglass: "M7 3h10M7 21h10M8 3v3l4 4 4-4V3M8 21v-3l4-4 4 4v3",
    squareterm: "M4 5h16v14H4zM8 10h4M8 14h6"
  };

  window.UI = {
    icon(name, cls) {
      if (name === "loader") {
        return `<svg class="ic ic-spin ${cls || ""}" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8.6" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-dasharray="5 7.4"/></svg>`;
      }
      const d = P[name] || P.dot;
      return `<svg class="ic ${cls || ""}" viewBox="0 0 24 24" aria-hidden="true"><path d="${d}"/></svg>`;
    },

    badge(kind, label) {
      const map = {
        success: ["is-success", "check"],
        failure: ["is-failure", "x"],
        blocked: ["is-blocked", "alert"],
        neutral: ["is-neutral", "dot"],
        running: ["is-neutral", "loader"],
        unknown: ["is-neutral", "help"]
      };
      const [cls, ic] = map[kind] || map.neutral;
      return `<span class="badge ${cls}">${UI.icon(ic)}${label}</span>`;
    },

    btn(opts) {
      const o = typeof opts === "string" ? { label: opts } : opts;
      const v = o.variant ? `is-${o.variant}` : "";
      const sz = o.size === "sm" ? "sm" : "";
      const dis = o.disabled ? "disabled" : "";
      const attrs = o.action ? `data-action="${o.action}"` : "";
      const data = Object.entries(o.data || {}).map(([k, vv]) => `data-${k}="${vv}"`).join(" ");
      const ico = o.icon ? UI.icon(o.icon) : "";
      return `<button class="btn ${v} ${sz}" ${attrs} ${data} ${dis} ${o.title ? `title="${o.title}"` : ""}>${ico}${o.label}</button>`;
    },

    iconBtn(icon, action, label, extra) {
      return `<button class="icon-btn" data-action="${action}" aria-label="${label}" title="${label}" ${extra || ""}>${UI.icon(icon)}</button>`;
    },

    winControls() {
      const line = (d) => `<svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="${d}" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="square"/></svg>`;
      return `<div class="win-controls">
        <button class="win-btn" data-action="win-minimize" aria-label="最小化" title="最小化">${line("M0 5h10")}</button>
        <button class="win-btn" data-action="win-maximize" aria-label="最大化" title="最大化">${line("M0.5 0.5h9v9h-9z")}</button>
        <button class="win-btn close" data-action="win-close" aria-label="关闭" title="关闭">${line("M0.5 0.5l9 9M9.5 0.5l-9 9")}</button>
      </div>`;
    },

    subStateLabel(s) {
      return ({
        success: ["success", "成功"],
        running: ["running", "执行中"],
        "waiting-dependency": ["neutral", "等待依赖"],
        "waiting-approval": ["blocked", "等待审批"],
        failure: ["failure", "失败"],
        blocked: ["blocked", "已阻塞"],
        skipped: ["neutral", "已跳过"],
        pending: ["neutral", "未开始"]
      })[s] || ["neutral", s];
    },

    taskStateLabel(s) {
      return ({
        running: ["running", "执行中"],
        queued: ["neutral", "排队中"],
        "waiting-approval": ["blocked", "等待审批"],
        "pause-pending": ["neutral", "等待暂停"],
        paused: ["neutral", "已暂停"],
        "stop-pending": ["unknown", "正在请求停止"],
        stopped: ["neutral", "已停止"],
        "needs-attention": ["blocked", "需要处理"],
        partial: ["blocked", "部分完成"],
        done: ["success", "已完成"],
        failed: ["failure", "失败"],
        unknown: ["unknown", "状态待确认"]
      })[s] || ["neutral", s];
    },

    esc(s) {
      return String(s == null ? "" : s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
    }
  };
})();
