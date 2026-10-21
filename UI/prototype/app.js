/* VocaMesh prototype · state, router, actions, keyboard — full interaction layer */

(function () {
  const clone = (v) => JSON.parse(JSON.stringify(v));

  const INITIAL = {
    screen: "welcome",
    theme: "light",
    themePref: "跟随系统",
    density: "comfortable",
    motion: "normal",
    grayscale: false,
    platform: "mac",
    closeBehavior: "window",
    concurrency: 3,
    autoRetry: false,
    logRetention: "30 天",
    notifPrefs: { approval: true, failure: true, done: false, systemOnly: false },

    taskId: "t1",
    taskFocus: "flow",
    inspectorOpen: false,
    inspectorTab: "logs",
    selectedSubtask: "s3",
    logQuery: "",
    logFilter: "全部",
    autoscroll: true,
    logsMore: 0,
    highlight: null,

    approvalId: "a1",
    approvalFilter: "待处理",
    agentId: "r1",
    roleAdvanced: false,
    roleExec: false,
    teamId: "tm1",
    teamView: "graph",
    teamWarning: "",
    graphZoom: 1,
    graphPan: { x: 0, y: 0 },
    graphPos: {},
    graphSel: null,
    memberFilter: "",
    wsId: "w2",
    runId: "#1244",
    settingsSection: "外观",
    helpArticle: "开始你的第一项任务",
    helpQuery: "",
    notifFilter: "全部",

    composerText: "",
    composerWs: "research-2025",
    composerMode: "选择团队",
    attach: [],

    filters: { taskState: "全部", taskWs: "全部", taskWho: "全部", taskTime: "全部", runType: "全部 Run", runResult: "全部", runTime: "最近 30 天" },

    dialog: null,
    dialogData: {},
    menu: null,
    palette: false,
    paletteQuery: "",
    paletteIndex: 0,
    scanning: false,
    toasts: [],
    feedback: [],
    downloads: [],
    defaultTool: "claude",
    historyMore: 0,

    tasks: clone(DATA.tasks),
    approvals: clone(DATA.approvals),
    notifications: clone(DATA.notifications),
    tools: clone(DATA.tools),
    agents: clone(DATA.agents),
    teams: clone(DATA.teams),
    workspaces: clone(DATA.workspaces)
  };

  const st = clone(INITIAL);
  window.STATE = st;
  window.__ACTIONS = null;

  /* ---------- infrastructure ---------- */
  let toastId = 0;
  function toast(text, kind) {
    if (st.toasts.some(t => t.text === text)) return;
    const id = ++toastId;
    st.toasts.push({ id, text, kind });
    if (st.toasts.length > 3) st.toasts = st.toasts.slice(-3);
    render();
    setTimeout(() => {
      st.toasts = st.toasts.filter(t => t.id !== id);
      render();
    }, 4200);
  }

  function later(ms, fn) {
    setTimeout(() => { fn(); render(); }, ms);
  }

  function currentTask() {
    return st.tasks.find(t => t.id === st.taskId) || st.tasks[0];
  }

  function copyText(text, label) {
    const done = () => toast((label || "已复制") + "：" + (text.length > 40 ? text.slice(0, 40) + "…" : text), "success");
    try {
      if (navigator.clipboard && window.isSecureContext) {
        navigator.clipboard.writeText(text).then(done).catch(() => fallback());
      } else fallback();
    } catch (e) { fallback(); }
    function fallback() {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand("copy"); done(); } catch (e) { toast("当前环境不允许写入剪贴板，已显示内容：\n" + text, "neutral"); }
      document.body.removeChild(ta);
    }
  }

  function download(filename, content, mime) {
    try {
      const blob = new Blob([content], { type: mime || "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 2000);
      st.downloads.push({ filename, at: new Date().toLocaleTimeString() });
      toast("已生成文件 " + filename + "（模拟导出内容）", "success");
    } catch (e) {
      toast("导出失败：" + e.message, "failure");
    }
  }

  let focusAfter = null;
  function render() {
    const root = document.getElementById("root");
    const screen = window.SCREENS[st.screen] || window.SCREENS.dashboard;
    document.body.classList.toggle("is-grayscale", st.grayscale);
    document.documentElement.setAttribute("data-theme", st.theme);
    root.innerHTML = screen(st);
    if (window.GRAPH && st.screen === "teams" && st.teamView === "graph") window.GRAPH.mount(st);
    try { history.replaceState(null, "", "#/" + st.screen); } catch (e) { /* file:// 可能受限 */ }
    if (focusAfter) {
      const t = document.querySelector(focusAfter);
      if (t) t.focus();
      focusAfter = null;
    }
  }
  window.renderApp = render;

  /* 供图形控制器（graph.js）使用的小 API */
  window.APP = {
    toast,
    render,
    state: st,
    dialog(name, data) { st.dialog = name; st.dialogData = data || {}; render(); },
    action(name, el) { const fn = A[name]; if (fn) { fn(el || { dataset: {} }); render(); } },
    patch(fn) { fn(st); render(); }
  };

  /* ---------- menus ---------- */
  const MENUS = {
    workspace: () => st.workspaces.map(w => ({ label: w.name + "  ·  " + w.path, action: "set-workspace", data: { value: w.name }, checked: st.composerWs === w.name })),
    "task-state": () => ["全部", "执行中", "等待审批", "排队中", "已暂停", "需要处理", "已完成", "失败"].map(v => ({ label: v, action: "set-filter", data: { field: "taskState", value: v }, checked: st.filters.taskState === v })),
    "task-ws": () => ["全部"].concat(st.workspaces.map(w => w.name)).map(v => ({ label: v, action: "set-filter", data: { field: "taskWs", value: v }, checked: st.filters.taskWs === v })),
    "task-who": () => ["全部"].concat(st.teams.map(t => t.name)).map(v => ({ label: v, action: "set-filter", data: { field: "taskWho", value: v }, checked: st.filters.taskWho === v })),
    "task-time": () => ["最近 7 天", "最近 30 天", "全部时间"].map(v => ({ label: v, action: "set-filter", data: { field: "taskTime", value: v }, checked: st.filters.taskTime === v })),
    "run-type": () => ["全部 Run", "仅失败", "仅审批相关", "配置变更"].map(v => ({ label: v, action: "set-filter", data: { field: "runType", value: v }, checked: st.filters.runType === v })),
    "run-result": () => ["全部", "已完成", "失败", "部分完成", "已停止", "状态未知"].map(v => ({ label: v, action: "set-filter", data: { field: "runResult", value: v }, checked: st.filters.runResult === v })),
    "run-time": () => ["最近 24 小时", "最近 7 天", "最近 30 天"].map(v => ({ label: v, action: "set-filter", data: { field: "runTime", value: v }, checked: st.filters.runTime === v })),
    "row-task": () => {
      const t = st.tasks.find(x => x.id === st.menu.id);
      if (!t) return [];
      const running = ["running", "pause-pending"].includes(t.state);
      return [
        { label: "打开任务详情", action: "open-task", data: { id: t.id } },
        { label: running ? "暂停任务" : "继续任务", action: running ? "row-pause" : "row-resume", data: { id: t.id } },
        { label: "复制 Run 编号", action: "copy-run", data: { id: t.id } },
        { label: "复制任务摘要", action: "copy-task", data: { id: t.id } },
        { label: "归档任务（演示）", action: "archive-task", data: { id: t.id } }
      ];
    },
    "row-run": () => {
      const r = DATA.runs.find(x => x.id === (st.menu && st.menu.id)) || DATA.runs[0];
      return [
        { label: "查看任务", action: "open-run-task", data: { id: r.id } },
        { label: "复制 Run 编号", action: "copy-text", data: { text: r.id, label: "Run 编号" } },
        { label: "导出该 Run 日志", action: "export-run-log", data: { id: r.id } },
        { label: "在新 Run 中重试", action: "retry-new-run", data: { id: r.id } }
      ];
    },
    "role-tool": () => st.tools.map(t => ({
      label: t.name + "（" + ({ ready: "可用", "auth-unknown": "需登录", missing: "未安装", undetectable: "无法检测" }[t.state]) + "）",
      action: "set-agent-tool", data: { id: t.id }, checked: (st.agents.find(a => a.id === st.agentId) || {}).toolId === t.id
    })),
    "add-member": () => st.agents.map(a => ({ label: a.name + " · " + a.role, action: "do-add-member", data: { name: a.name } })),
    attach: () => ["竞品资料汇总.md", "pricing-2025q3.csv", "会议记录.txt"].map(f => ({ label: f, action: "attach-file", data: { file: f } })),
    "dialog-agent": () => st.agents.map(a => ({ label: a.name + " · " + a.tool, action: "set-dialog-agent", data: { value: a.name + " · " + a.tool }, checked: (st.dialogAgent || "").indexOf(a.name) === 0 })),
    "dialog-team": () => st.teams.map(t => ({ label: t.name + " · " + t.members.length + " 名成员 · 主管：" + t.supervisor, action: "set-dialog-team", data: { value: t.name + " · " + t.members.length + " 名成员 · 主管：" + t.supervisor }, checked: (st.dialogTeam || "").indexOf(t.name) === 0 })),
    retention: () => ["7 天", "30 天", "90 天", "永久保留"].map(v => ({ label: v, action: "set-retention", data: { value: v }, checked: st.logRetention === v })),
    "task-more": () => [
      { label: "复制任务链接", action: "copy-task-link", data: { id: st.taskId } },
      { label: "导出执行摘要", action: "export-task-summary", data: { id: st.taskId } },
      { label: "在历史记录中查看", action: "go", data: { screen: "history" } },
      { label: "归档任务（演示）", action: "archive-task", data: { id: st.taskId } }
    ],
    "graph-node": () => {
      const team = st.teams.find(t => t.id === st.teamId) || st.teams[0];
      const name = st.menu.id;
      const m = team && team.members.find(x => x.name === name);
      const items = [
        { label: "编辑 Agent 信息", action: "graph-edit", data: { name } },
        { label: "添加下属", action: "graph-add-child", data: { name } },
        { label: "添加直属上级…", action: "graph-open-superior", data: { name } }
      ];
      if (m && name !== team.supervisor) items.push({ label: "设为团队主管", action: "graph-make-supervisor", data: { name } });
      items.push({ label: "从此团队移除", action: "graph-remove", data: { name } });
      return items;
    },
    "member-actions": () => [
      { label: "设为直属主管", action: "set-member-supervisor", data: { name: st.menu.id } },
      { label: "从此团队移除", action: "remove-member", data: { name: st.menu.id } }
    ]
  };

  function openMenu(kind, el, id) {
    st.menu = { kind, id, items: [] };
    const items = MENUS[kind] ? MENUS[kind]() : [];
    if (!items.length) { st.menu = null; toast("没有可用的菜单项", "neutral"); return; }
    const r = el.getBoundingClientRect();
    const width = 240;
    const x = Math.min(Math.max(8, r.left), window.innerWidth - width - 8);
    const y = Math.min(r.bottom + 4, window.innerHeight - Math.min(items.length * 32 + 16, 320));
    st.menu = { kind, id, items, x, y, width };
  }

  /* ---------- actions ---------- */
  const A = {};

  A.go = (el) => {
    const screen = el.dataset.screen;
    if (!screen) return;
    st.screen = screen;
    st.dialog = null;
    st.palette = false;
    st.menu = null;
    if (screen !== "task") st.inspectorOpen = false;
  };

  A["go-notifications"] = () => { st.screen = "notifications"; st.menu = null; };
  A.palette = () => { st.palette = true; st.paletteQuery = ""; st.paletteIndex = 0; focusAfter = "#palette-input"; };
  A["close-palette"] = () => { st.palette = false; };
  A["palette-move"] = (el) => {
    const dir = Number(el.dataset.dir);
    const items = DATA.paletteCommands.filter(c => !st.paletteQuery || c.label.toLowerCase().includes(st.paletteQuery.toLowerCase()));
    if (!items.length) return;
    st.paletteIndex = (st.paletteIndex + dir + items.length) % items.length;
  };
  A["palette-run"] = (el) => {
    const items = DATA.paletteCommands.filter(c => !st.paletteQuery || c.label.toLowerCase().includes(st.paletteQuery.toLowerCase()));
    const item = el.dataset.index != null ? items[Number(el.dataset.index)] : items[st.paletteIndex];
    if (!item) return;
    st.palette = false;
    if (item.action) {
      const fake = { dataset: {} };
      A[item.action](fake);
    } else {
      st.screen = item.screen;
    }
  };
  A["toggle-theme"] = () => {
    st.theme = st.theme === "dark" ? "light" : "dark";
    st.themePref = st.theme === "dark" ? "暗色" : "亮色";
  };
  A["theme-pref"] = (el) => {
    st.themePref = el.dataset.pref;
    if (st.themePref === "亮色") st.theme = "light";
    else if (st.themePref === "暗色") st.theme = "dark";
    else st.theme = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  };
  A["density-pref"] = (el) => { st.density = el.dataset.value; };
  A.platform = (el) => { st.platform = el.dataset.value; };
  A["toggle-motion"] = () => { st.motion = st.motion === "reduced" ? "normal" : "reduced"; };
  A["toggle-grayscale"] = () => { st.grayscale = !st.grayscale; };
  A["close-behavior"] = (el) => { st.closeBehavior = el.dataset.value; };
  A.concurrency = (el) => {
    st.concurrency = Math.max(1, Math.min(6, st.concurrency + Number(el.dataset.delta)));
    toast("并发上限已设为 " + st.concurrency + "（演示）", "neutral");
  };
  A["toggle-autoretry"] = () => { st.autoRetry = !st.autoRetry; };
  A["toggle-notif-pref"] = (el) => { st.notifPrefs[el.dataset.key] = !st.notifPrefs[el.dataset.key]; };
  A["reset-appearance"] = () => {
    st.theme = "light"; st.themePref = "跟随系统"; st.density = "comfortable"; st.grayscale = false; st.motion = "normal"; st.platform = "mac";
    toast("已恢复默认外观", "success");
  };
  A["reset-demo"] = () => {
    const keep = { toasts: st.toasts, downloads: st.downloads };
    Object.assign(st, clone(INITIAL), keep);
    toast("演示数据已重置", "success");
  };
  A["show-shortcuts"] = () => { st.dialog = "shortcuts"; };
  A["close-dialog"] = () => { st.dialog = null; st.dialogData = {}; };
  A["close-menu"] = () => { st.menu = null; };

  A["menu-workspace"] = (el) => openMenu("workspace", el);
  A["set-workspace"] = (el) => { st.composerWs = el.dataset.value; st.menu = null; toast("工作空间：" + el.dataset.value, "neutral"); };
  A["menu-filter"] = (el) => openMenu(el.dataset.kind, el);
  A["set-filter"] = (el) => {
    st.filters[el.dataset.field] = el.dataset.value;
    st.menu = null;
  };
  A["menu-row-task"] = (el) => openMenu("row-task", el, el.dataset.id);
  A["menu-task-more"] = (el) => openMenu("task-more", el);
  A["menu-add-member"] = (el) => openMenu("add-member", el);
  A["menu-attach"] = (el) => openMenu("attach", el);
  A["menu-role-tool"] = (el) => openMenu("role-tool", el);
  A["menu-retention"] = (el) => openMenu("retention", el);
  A["menu-run"] = (el) => openMenu("row-run", el, el.dataset.id);

  A["focus-composer"] = () => { st.screen = "dashboard"; focusAfter = "#composer-input"; };

  A["dialog-templates"] = () => { st.dialog = "templates"; };
  A["use-template"] = (el) => {
    st.composerText = el.dataset.text;
    st.composerMode = el.dataset.mode || "选择团队";
    st.dialog = null;
    st.screen = "dashboard";
    focusAfter = "#composer-input";
    toast("模板已填入输入框，可继续修改", "success");
  };

  A["dialog-create-task"] = () => { st.dialog = "create-task"; };
  A["set-mode"] = (el) => { st.composerMode = el.dataset.mode; };

  A["menu-attach"] = (el) => openMenu("attach", el);
  A["attach-file"] = (el) => {
    if (!st.attach.includes(el.dataset.file)) st.attach.push(el.dataset.file);
    st.menu = null;
    toast("已附加 " + el.dataset.file + "（模拟引用）", "success");
  };
  A["remove-attach"] = (el) => { st.attach = st.attach.filter(f => f !== el.dataset.file); };

  A["create-task"] = () => {
    const ta = document.getElementById("composer-input") || document.getElementById("dlg-desc");
    const text = (ta && ta.value.trim()) || st.composerText || "整理 Q3 竞品资料并生成对比报告";
    const attach = st.attach.slice();
    st.composerText = "";
    st.attach = [];
    st.dialog = null;
    const id = "t" + (st.tasks.length + 1);
    const team = st.composerMode === "快速执行" ? "研究员 · 单 Agent" : st.composerMode === "选择团队" ? "调研团队 · 4 人" : "自定义执行";
    st.tasks.unshift({
      id, name: text.length > 42 ? text.slice(0, 42) + "…" : text,
      ws: st.composerWs, team, run: "#" + (1250 + st.tasks.length), supervisor: st.composerMode === "快速执行" ? "研究员" : "研究协调员",
      started: "现在", state: "queued", stateNote: "已创建 · 等待运行环境检查",
      subtasks: st.composerMode === "快速执行"
        ? [{ id: "s1", name: "执行任务", agent: "研究员 · Claude Code", state: "pending", result: "未开始" }]
        : [{ id: "s1", name: "拆解任务并分配角色", agent: "研究协调员 · 主管", state: "pending", result: "未开始" }],
      messages: attach.length ? [{ who: "me", name: "你", time: "现在", text: "已附加 " + attach.length + " 个文件（模拟）：" + attach.join("、") }] : [],
      instruction: null
    });
    st.taskId = id;
    toast("任务已创建 · 状态：排队中（尚未开始执行）", "success");
    later(1400, () => {
      const t = st.tasks.find(x => x.id === id);
      if (!t) return;
      t.state = "running";
      t.stateNote = "运行环境检查通过 · 已开始执行";
      toast("运行环境检查通过，任务已开始执行", "neutral");
    });
  };

  A["open-task"] = (el) => {
    st.taskId = el.dataset.id;
    st.screen = "task";
    st.inspectorOpen = false;
    st.dialog = null;
    st.menu = null;
  };
  A["metric"] = (el) => {
    st.filters.taskState = el.dataset.state;
    st.screen = "tasks";
  };

  A["row-pause"] = () => {
    const t = st.tasks.find(x => x.id === st.menu.id);
    st.menu = null;
    if (!t) return;
    t.state = "paused";
    t.stateNote = "已暂停（演示：批量操作直接到达安全边界）";
    toast("已暂停：" + t.name, "neutral");
  };
  A["row-resume"] = () => {
    const t = st.tasks.find(x => x.id === st.menu.id);
    st.menu = null;
    if (!t) return;
    t.state = "running";
    t.stateNote = "继续执行中";
    toast("已继续：" + t.name, "success");
  };
  A["copy-run"] = () => {
    const t = st.tasks.find(x => x.id === st.menu.id);
    st.menu = null;
    if (t) copyText(t.run, "Run 编号");
  };
  A["copy-task"] = () => {
    const t = st.tasks.find(x => x.id === st.menu.id);
    st.menu = null;
    if (t) copyText(t.name + " · " + t.ws + " · " + t.run, "任务摘要");
  };
  A["copy-task-link"] = (el) => {
    st.menu = null;
    copyText("vocamesh://task/" + el.dataset.id, "任务链接");
  };
  A["archive-task"] = (el) => {
    const id = (st.menu && st.menu.id) || el.dataset.id;
    st.menu = null;
    st.tasks = st.tasks.filter(t => t.id !== id);
    if (st.taskId === id) { st.taskId = (st.tasks[0] || {}).id; st.screen = "tasks"; }
    toast("任务已归档（演示：从列表移除）", "neutral");
  };
  A["copy-text"] = (el) => { st.menu = null; copyText(el.dataset.text, el.dataset.label || "已复制"); };
  A["export-run-log"] = (el) => {
    st.menu = null;
    const lines = DATA.logs.map(l => l.join("  ")).join("\n");
    download("run-" + String(el.dataset.id).replace("#", "") + ".log", lines, "text/plain;charset=utf-8");
  };
  A["open-run-task"] = (el) => {
    st.menu = null;
    const run = DATA.runs.find(r => r.id === el.dataset.id);
    const t = st.tasks.find(x => run && x.run === run.id) || st.tasks[0];
    st.taskId = t.id;
    st.screen = "task";
  };
  A["export-task-summary"] = (el) => {
    st.menu = null;
    const t = st.tasks.find(x => x.id === el.dataset.id) || currentTask();
    const md = ["# " + t.name, "", "- 工作空间：" + t.ws, "- 团队：" + t.team, "- Run：" + t.run, "- 状态：" + t.stateNote, "", "## 子任务"].concat(t.subtasks.map(s => "- [" + s.state + "] " + s.name + " · " + s.agent + " · " + s.result)).join("\n");
    download("task-" + t.id + "-summary.md", md, "text/markdown;charset=utf-8");
  };
  A["export-history"] = () => {
    const csv = ["run,task,executor,state,start,duration"].concat(DATA.runs.map(r => [r.id, '"' + r.task + '"', r.who, r.state, r.start, r.dur].join(","))).join("\n");
    download("vocamesh-runs.csv", csv, "text/csv;charset=utf-8");
  };
  A["export-partial"] = () => {
    const rows = ["path,status,note"];
    DATA.partialRun.groups.forEach(g => g.items.forEach(([f, d]) => rows.push([f, g.label, '"' + d + '"'].join(","))));
    download("run-1244-partial.csv", rows.join("\n"), "text/csv;charset=utf-8");
  };
  A["history-more"] = () => {
    st.historyMore += 3;
    toast("已加载 " + st.historyMore + " 条更早的 Run（演示）", "neutral");
  };

  A["select-agent"] = (el) => { st.agentId = el.dataset.id; };
  A["select-team"] = (el) => { st.teamId = el.dataset.id; st.teamWarning = ""; };
  A["select-ws"] = (el) => { st.wsId = el.dataset.id; };
  A["select-run"] = (el) => { st.runId = el.dataset.id; };
  A["select-approval"] = (el) => { st.approvalId = el.dataset.id; };
  A["approval-filter"] = (el) => { st.approvalFilter = el.dataset.filter; };

  A["task-focus"] = (el) => { st.taskFocus = el.dataset.focus; };
  A["clear-task-filter"] = () => { st.filters = Object.assign({}, st.filters, { taskState: "全部", taskWs: "全部", taskWho: "全部", taskTime: "全部" }); st.taskQuery = ""; st.menu = null; };
  A["clear-run-filter"] = () => { st.filters = Object.assign({}, st.filters, { runType: "全部 Run", runResult: "全部", runTime: "最近 30 天" }); };

  A["open-subtask"] = (el) => {
    st.selectedSubtask = el.dataset.id;
    st.inspectorOpen = true;
    st.inspectorTab = "logs";
    st.logsMore = 0;
  };
  A["inspector-open"] = (el) => { st.inspectorOpen = true; if (el && el.dataset.tab) st.inspectorTab = el.dataset.tab; };
  A["inspector-close"] = () => { st.inspectorOpen = false; };
  A["inspector-tab"] = (el) => { st.inspectorTab = el.dataset.tab; };
  A["toggle-autoscroll"] = () => { st.autoscroll = !st.autoscroll; };
  A["log-filter"] = (el) => { st.logFilter = el.dataset.filter; };
  A["logs-more"] = () => {
    if (st.logsMore >= 12) { toast("已到最早的可见日志（演示上限）", "neutral"); return; }
    st.logsMore += 4;
    toast("已加载更早的日志（演示）", "neutral");
  };
  A["copy-logs"] = () => {
    const q = (st.logQuery || "").toLowerCase();
    const lines = DATA.logs.filter(l => !q || l.join(" ").toLowerCase().includes(q)).map(l => l.join("  "));
    copyText(lines.join("\n"), "日志（" + lines.length + " 行）");
  };
  A["locate-subtask"] = () => {
    st.highlight = st.selectedSubtask;
    st.inspectorOpen = false;
    toast("已在执行流程中定位该子任务", "success");
    later(2400, () => { st.highlight = null; });
  };
  A["file-preview"] = (el) => { st.dialog = "file-preview"; st.dialogData = { path: el.dataset.path }; };

  /* task run controls */
  A["pause-task"] = () => {
    const t = currentTask();
    if (!["running", "waiting-approval"].includes(t.state)) return;
    t.state = "pause-pending";
    t.stateNote = "等待暂停：已停止派发新工作";
    toast("已停止派发新工作；当前执行将在安全边界后暂停", "blocked");
    later(1700, () => {
      if (t.state !== "pause-pending") return;
      t.state = "paused";
      t.stateNote = "已暂停 · 全部执行到达安全边界";
      toast("已暂停：所有执行均已到达安全边界", "neutral");
    });
  };

  A["stop-task"] = () => {
    const t = currentTask();
    t.state = "stop-pending";
    t.stateNote = "正在请求停止 · 等待运行时确认";
    toast("正在请求停止：已向 2 个运行时发出请求，尚未确认", "neutral");
    later(1800, () => {
      if (t.state !== "stop-pending") return;
      t.state = "stopped";
      t.stateNote = "已停止 · 2 个运行时已确认";
      toast("已停止：2 个运行时已确认，已有成果保留", "neutral");
    });
  };

  A["resume-task"] = () => {
    const t = currentTask();
    t.state = "running";
    t.stateNote = "继续 · 已通过权限与依赖检查";
    toast("继续：权限、依赖与输入版本检查通过，恢复调度", "success");
  };

  A["retry-failed"] = () => {
    const t = currentTask();
    t.state = "running";
    t.stateNote = "新 Run 已启动 · 旧 Run 记录保留";
    t.subtasks = t.subtasks.map(s => (["failure", "blocked", "skipped"].includes(s.state) ? Object.assign({}, s, { state: "running", result: "重试中（新 Run）" }) : s));
    const oldRun = t.run;
    t.run = "#" + (1260 + Math.floor(Math.random() * 30));
    toast("已创建新的 Run " + t.run + "：仅重试失败部分；旧 Run " + oldRun + " 的日志与产物保持不变", "success");
  };

  A["skip-partial"] = () => {
    const t = currentTask();
    t.state = "partial";
    t.stateNote = "部分完成 · 已标注跳过范围";
    t.subtasks = t.subtasks.map(s => (["failure", "blocked", "waiting-approval"].includes(s.state) ? Object.assign({}, s, { state: "skipped", result: "已跳过 · 结果标注为不完整" }) : s));
    toast("已生成部分成果：完成项与跳过项已分组标注，整体不算成功", "blocked");
  };

  A["retry-new-run"] = () => {
    st.menu = null;
    const id = "#" + (1260 + Math.floor(Math.random() * 30));
    DATA.runs.unshift({ id, task: "批量规范化 320 条术语词表", who: "数据小组", state: "running", start: "现在", dur: "—" });
    toast("已创建 Run " + id + "；#1244 的日志与产物保持不变", "success");
  };

  A["allow-approval"] = (el) => {
    const a = st.approvals.find(x => x.id === el.dataset.id);
    if (!a || a.state !== "pending") return;
    a.state = "allowed";
    a.history.push(["现在", "已允许一次（你）", "作用域：仅本文件 · 仅本次 · 仅本 Run"]);
    const t = st.tasks.find(x => x.id === a.taskId);
    if (t) {
      t.state = "running";
      t.stateNote = "已获得一次性授权，继续执行";
      const s = t.subtasks.find(x => x.state === "waiting-approval");
      if (s) { s.state = "running"; s.result = "已获授权，正在写入"; }
      t.messages.push({ who: "supervisor", name: t.supervisor + " · 主管", time: "现在", text: "已收到你的“仅此一次允许”。我继续第 4 步写入，完成后会汇总结论。" });
    }
    toast("已记录决定：仅此一次允许（不可用于其他文件）", "success");
  };

  A["deny-approval"] = (el) => {
    const a = st.approvals.find(x => x.id === el.dataset.id);
    if (!a || a.state !== "pending") return;
    a.state = "denied";
    a.history.push(["现在", "已拒绝（你）", "拒绝后不会重试；需要重新发起合规申请"]);
    const t = st.tasks.find(x => x.id === a.taskId);
    if (t) {
      t.state = "needs-attention";
      t.stateNote = "审批被拒绝 · 相关步骤未执行";
      const s = t.subtasks.find(x => x.state === "waiting-approval");
      if (s) { s.state = "blocked"; s.result = "授权被拒绝，未执行"; }
      t.messages.push({ who: "supervisor", name: t.supervisor + " · 主管", time: "现在", text: "已收到拒绝。该步骤不会执行，我也不会绕过你的决定重试。你可以选择跳过该步骤生成部分成果，或停止任务。" });
    }
    toast("已拒绝该请求 · 任务停在当前步骤，不会自动重试", "blocked");
  };

  A["send-message"] = () => {
    const t = currentTask();
    const ta = document.getElementById("task-input");
    const text = (ta && ta.value.trim()) || "";
    if (!text) { toast("请先输入内容", "neutral"); return; }
    if (st.instrTarget) {
      t.instruction = { target: st.instrTarget, text, state: "queued", impact: "该指令可能改变下一步计划：建议先做变更影响预览" };
      toast("定向指令已提交 · 排队待投递（将同步主管）", "neutral");
      later(1100, () => { if (t.instruction) { t.instruction.state = "delivered"; toast("已送达：运行时确认收到（尚不代表已应用）", "neutral"); } });
      later(2300, () => { if (t.instruction) { t.instruction.state = "waiting-point"; toast("CLI 不支持执行中增量输入，等待安全投递点", "blocked"); } });
      later(3600, () => { if (t.instruction) { t.instruction.state = "applied"; toast("已应用：下一阶段生效", "success"); } });
    } else {
      t.messages.push({ who: "me", name: "你", time: "现在", text });
      toast("消息已发送给主管", "success");
      later(1000, () => {
        t.messages.push({ who: "supervisor", name: t.supervisor + " · 主管", time: "现在", text: "收到。我会按“来源可追溯、缺数据留空”的规则继续，需要授权时会先停下询问。" });
      });
    }
    st.instrTarget = null;
  };

  A["toggle-instruction-target"] = () => {
    st.instrTarget = st.instrTarget ? null : "分析师";
    toast(st.instrTarget ? "定向指令模式：目标 分析师（会同步主管）" : "已切换为发送给主管", "neutral");
  };

  A["toggle-perm"] = (el) => {
    const a = st.agents.find(x => x.id === st.agentId);
    if (!a) return;
    a.perms[el.dataset.key] = !a.perms[el.dataset.key];
    toast("权限已" + (a.perms[el.dataset.key] ? "开启" : "关闭") + "：" + el.dataset.key + "（从下一个任务生效）", "neutral");
  };
  A["toggle-advanced-role"] = () => { st.roleAdvanced = !st.roleAdvanced; };
  A["toggle-role-exec"] = () => { st.roleExec = !st.roleExec; };
  A["save-role"] = () => {
    const nameEl = document.getElementById("role-name");
    const descEl = document.getElementById("role-desc");
    const a = st.agents.find(x => x.id === st.agentId);
    if (a) {
      if (nameEl) a.name = nameEl.value.trim() || a.name;
      if (descEl) a.desc = descEl.value;
    }
    toast("配置已保存（角色未运行；改动从下一个新任务生效）", "success");
  };
  A["confirm-delete-role"] = (el) => {
    st.dialog = "confirm-delete-role";
    st.dialogData = { id: el.dataset.id };
  };
  A["do-delete-role"] = () => {
    const id = st.dialogData.id;
    const a = st.agents.find(x => x.id === id);
    st.agents = st.agents.filter(x => x.id !== id);
    st.agentId = (st.agents[0] || {}).id;
    st.dialog = null;
    toast("已删除角色 " + (a ? a.name : "") + "（历史 Run 仍可追溯）", "blocked");
  };
  A["set-agent-tool"] = (el) => {
    const a = st.agents.find(x => x.id === st.agentId);
    const tool = st.tools.find(t => t.id === el.dataset.id);
    st.menu = null;
    if (!a || !tool) return;
    a.toolId = tool.id;
    a.tool = tool.name + " · " + tool.version;
    a.state = tool.state === "ready" ? "ready" : tool.state === "auth-unknown" ? "auth-unknown" : "not-ready";
    toast("执行工具已设为 " + tool.name + " · 角色状态：" + (a.state === "ready" ? "可用" : "未就绪"), a.state === "ready" ? "success" : "blocked");
  };

  A["dialog-new-role"] = () => { st.dialog = "new-role"; st.newRoleTool = st.newRoleTool || "claude"; };
  A["pick-new-role-tool"] = (el) => { st.newRoleTool = el.dataset.id; };
  A.noop = () => {};
  A["save-new-role"] = () => {
    const name = (document.getElementById("nr-name") || {}).value || "未命名角色";
    const desc = (document.getElementById("nr-desc") || {}).value || "未填写职责描述";
    const tool = st.tools.find(t => t.id === st.newRoleTool) || st.tools[0];
    st.agents.push({
      id: "r" + (st.agents.length + 1), name, role: tool.name, tool: tool.name + " · " + tool.version, toolId: tool.id,
      state: tool.state === "ready" ? "not-ready" : "not-ready", desc, instr: "（尚未填写角色指令）", perms: { read: true, write: false, shell: false, net: false }
    });
    st.agentId = st.agents[st.agents.length - 1].id;
    st.dialog = null;
    toast("角色配置已保存 · 状态：未就绪（完成认证后才能执行任务）", "success");
  };

  A["dialog-new-team"] = () => { st.dialog = "new-team"; };
  A["save-team"] = () => {
    const name = (document.getElementById("nt-name") || {}).value || "新团队";
    const sup = (document.getElementById("nt-sup") || {}).value || "研究协调员";
    st.teams.push({
      id: "tm" + (st.teams.length + 1), name, supervisor: sup, ws: "research-2025",
      members: [{ name: sup, role: "主管 · 向用户汇报", reports: "用户", tool: "Claude Code", state: "ready" }]
    });
    st.teamId = st.teams[st.teams.length - 1].id;
    st.dialog = null;
    toast("团队已保存 · 关系检查通过（无自我管理、无间接环）", "success");
  };
  A["do-add-member"] = (el) => {
    st.menu = null;
    const team = st.teams.find(t => t.id === st.teamId);
    const name = el.dataset.name;
    if (!team) return;
    if (name === team.supervisor) {
      st.teamWarning = "已阻止：把“" + name + "”设为下属会形成自我管理（A → A），或与现有直属关系构成间接环（A → B → A）。请先调整上级关系。";
      toast("已阻止：检测到自我管理 / 间接环", "blocked");
      later(6000, () => { if (st.teamWarning) st.teamWarning = ""; });
      return;
    }
    if (team.members.some(m => m.name === name)) { toast("该成员已在团队中", "blocked"); return; }
    const agent = st.agents.find(a => a.name === name);
    team.members.push({ name, role: (agent && agent.role) || "成员", reports: team.supervisor, tool: (agent && agent.tool) || "Claude Code", state: (agent && agent.state) || "ready" });
    st.teamWarning = "";
    toast("已添加成员：" + name + "（直属主管 " + team.supervisor + "）", "success");
  };
  A["menu-member"] = (el) => openMenu("member-actions", el, el.dataset.name);
  A["remove-member"] = () => {
    const team = st.teams.find(t => t.id === st.teamId);
    const name = st.menu.id;
    st.menu = null;
    if (!team) return;
    team.members = team.members.filter(m => m.name !== name);
    toast("已移除成员：" + name + "（其历史 Run 不受影响）", "neutral");
  };
  A["set-member-supervisor"] = () => {
    const team = st.teams.find(t => t.id === st.teamId);
    const name = st.menu.id;
    st.menu = null;
    const m = team && team.members.find(x => x.name === name);
    if (!m) return;
    m.reports = team.supervisor;
    toast(name + " 的直属主管已设为 " + team.supervisor, "success");
  };
  A["dialog-team-edit"] = () => { st.dialog = "team-edit"; st.dialogData = { teamId: st.teamId }; };
  A["save-team-edit"] = () => {
    const team = st.teams.find(t => t.id === st.dialogData.teamId);
    const nameEl = document.getElementById("te-name");
    const wsEl = document.getElementById("te-ws");
    if (team) {
      if (nameEl && nameEl.value.trim()) team.name = nameEl.value.trim();
      if (wsEl && wsEl.value.trim()) team.ws = wsEl.value.trim();
    }
    st.dialog = null;
    toast("团队设置已保存（权限覆盖不会影响正在运行的 Run）", "success");
  };

  A["team-view"] = (el) => { st.teamView = el.dataset.view; };

  /* ---- org chart ---- */
  const teamNow = () => st.teams.find(t => t.id === st.teamId) || st.teams[0];
  const cycleTo = (team, name, sup) => {
    let cur = sup, guard = 0;
    while (cur && cur !== "用户" && guard++ < 60) {
      if (cur === name) return true;
      const m = team.members.find(x => x.name === cur);
      cur = m ? m.reports : null;
    }
    return false;
  };
  A["graph-select"] = (el) => { st.graphSel = el.dataset.name; };
  A["graph-node-menu"] = (el) => openMenu("graph-node", el, el.dataset.name);
  A["graph-add-child"] = (el) => { st.dialog = "graph-new"; st.dialogData = { parent: el.dataset.name }; st.menu = null; };
  A["graph-new-agent"] = () => { st.dialog = "graph-new"; st.dialogData = { parent: st.graphSel || teamNow().supervisor }; };
  A["graph-edit"] = (el) => { st.dialog = "graph-edit"; st.dialogData = { name: el.dataset.name }; st.menu = null; };
  A["graph-open-superior"] = (el) => { st.dialog = "graph-superior"; st.dialogData = { name: el.dataset.name }; st.menu = null; };
  A["graph-make-supervisor"] = (el) => {
    st.menu = null;
    const team = teamNow();
    const name = el.dataset.name;
    const prev = team.supervisor;
    const m = team.members.find(x => x.name === name);
    if (!m) return;
    team.supervisor = name;
    m.reports = "用户";
    const old = team.members.find(x => x.name === prev);
    if (old && prev !== name) old.reports = name;
    toast("已将 " + name + " 设为团队主管（原主管 " + prev + " 改为其下属）", "success");
  };
  A["graph-remove"] = (el) => {
    st.menu = null;
    const team = teamNow();
    const name = el.dataset.name;
    if (name === team.supervisor) { toast("主管不能直接移除，请先指定新的主管", "blocked"); return; }
    team.members = team.members.filter(m => m.name !== name);
    team.members.forEach(m => { if (m.reports === name) m.reports = team.supervisor; });
    if (st.graphSel === name) st.graphSel = null;
    toast("已从团队移除 " + name + "（其历史 Run 不受影响）", "neutral");
  };
  A["graph-save-new"] = () => {
    const team = teamNow();
    const name = ((document.getElementById("gn-name") || {}).value || "").trim();
    const role = ((document.getElementById("gn-role") || {}).value || "成员").trim();
    const parent = (document.getElementById("gn-parent") || {}).value || st.dialogData.parent || team.supervisor;
    const toolId = (document.getElementById("gn-tool") || {}).value || "claude";
    if (!name) { toast("请填写 Agent 名称", "neutral"); return; }
    if (team.members.some(m => m.name === name)) { toast("该名称已存在于团队中", "blocked"); return; }
    const tool = st.tools.find(t => t.id === toolId) || st.tools[0];
    const agentState = tool.state === "ready" ? "ready" : tool.state === "auth-unknown" ? "auth-unknown" : "not-ready";
    if (!st.agents.some(a => a.name === name)) {
      st.agents.push({ id: "r" + (st.agents.length + 1), name, role, tool: tool.name + " · " + tool.version, toolId: tool.id, state: agentState, desc: "（从关系图创建）", instr: "（尚未填写角色指令）", perms: { read: true, write: false, shell: false, net: false } });
    }
    team.members.push({ name, role, reports: parent, tool: tool.name, state: agentState });
    st.graphSel = name;
    st.dialog = null;
    toast("已创建 " + name + "，直属主管：" + parent + "（角色窗口：未运行）", "success");
  };
  A["graph-save-edit"] = () => {
    const team = teamNow();
    const name = st.dialogData.name;
    const m = team.members.find(x => x.name === name);
    const newName = ((document.getElementById("ge-name") || {}).value || name).trim();
    const role = ((document.getElementById("ge-role") || {}).value || (m && m.role) || "").trim();
    if (!m) return;
    if (newName !== name && team.members.some(x => x.name === newName)) { toast("该名称已被占用", "blocked"); return; }
    team.members.forEach(x => { if (x.reports === name) x.reports = newName; });
    if (team.supervisor === name) team.supervisor = newName;
    m.name = newName; m.role = role;
    const ag = st.agents.find(a => a.name === name);
    if (ag) { ag.name = newName; ag.role = role; }
    st.graphSel = newName;
    st.dialog = null;
    toast("已保存 " + newName + " 的信息（不影响正在运行的 Run）", "success");
  };
  A["graph-save-superior"] = () => {
    const team = teamNow();
    const name = st.dialogData.name;
    const sel = document.querySelector('input[name="gs-sup"]:checked');
    const sup = sel ? sel.value : team.supervisor;
    const m = team.members.find(x => x.name === name);
    if (!m) return;
    if (sup === name) { toast("不能把自己设为直属上级（自我管理）", "blocked"); return; }
    if (cycleTo(team, name, sup)) { toast("已阻止：会产生间接环（" + name + " → " + sup + " → " + name + "）", "blocked"); return; }
    m.reports = sup;
    if (sup === "用户") { team.supervisor = name; }
    st.dialog = null;
    toast("已将 " + name + " 的直属上级设为 " + sup, "success");
  };
  A["graph-zoom"] = (el) => { window.GRAPH.zoomBy(st, Number(el.dataset.dir) * 0.1); };
  A["graph-fit"] = () => { window.GRAPH.fit(st, teamNow()); };
  A["graph-tidy"] = () => { window.GRAPH.tidy(st, teamNow()); };
  A["graph-reset-pos"] = () => { window.GRAPH.resetPositions(st); };

  A["verify-tool"] = (el) => {
    const tool = st.tools.find(t => t.id === el.dataset.id);
    if (!tool) return;
    toast("正在请求运行时状态…（模拟）", "neutral");
    later(1300, () => {
      if (tool.state === "auth-unknown") {
        tool.state = "ready";
        tool.note = "已验证登录 · 可用于任务执行";
        toast("认证已验证：" + tool.name + " 可用", "success");
      } else if (tool.state === "ready") {
        toast("验证通过：登录状态有效（模拟）", "success");
      } else {
        toast("无法确认状态：已记录，仍显示为“无法检测”", "blocked");
      }
    });
  };
  A["set-default-tool"] = (el) => {
    st.defaultTool = el.dataset.id;
    const t = st.tools.find(x => x.id === el.dataset.id);
    toast("已设为默认执行工具：" + (t ? t.name : ""), "success");
  };
  A["dialog-tool-config"] = (el) => { st.dialog = "tool-config"; st.dialogData = { id: el.dataset.id }; };
  A["dialog-tool-path"] = (el) => { st.dialog = "tool-path"; st.dialogData = { id: el.dataset.id }; };
  A["save-tool-path"] = () => {
    const p = (document.getElementById("tp-path") || {}).value || "";
    const tool = st.tools.find(t => t.id === st.dialogData.id);
    if (tool && p.trim()) {
      tool.path = p.trim();
      tool.state = "undetectable";
      tool.note = "已手动指定路径，等待验证（无法检测时不会假定可用）";
    }
    st.dialog = null;
    toast("路径已保存，状态为“无法检测”，请点击重新检测", "blocked");
  };
  A["dialog-install-guide"] = (el) => { st.dialog = "install-guide"; st.dialogData = { id: el.dataset.id }; };
  A["dialog-tool-error"] = (el) => { st.dialog = "tool-error"; st.dialogData = { id: el.dataset.id }; };

  A["menu-dialog-agent"] = (el) => openMenu("dialog-agent", el);
  A["set-dialog-agent"] = (el) => { st.dialogAgent = el.dataset.value; st.menu = null; };
  A["menu-dialog-team"] = (el) => openMenu("dialog-team", el);
  A["set-dialog-team"] = (el) => { st.dialogTeam = el.dataset.value; st.menu = null; };
  A["new-role-tool"] = (el) => {
    st.newRoleTool = { id: el.dataset.id, name: el.dataset.name };
    const t = st.tools.find(x => x.id === el.dataset.id);
    toast("已选择执行工具：" + el.dataset.name + (t && t.state !== "ready" ? "（该工具当前未就绪）" : ""), t && t.state === "ready" ? "success" : "blocked");
  };
  A["win-minimize"] = () => toast("最小化由桌面宿主处理（原型不控制真实窗口）", "neutral");
  A["win-maximize"] = () => toast("最大化由桌面宿主处理（原型不控制真实窗口）", "neutral");
  A["win-close"] = () => {
    if (st.closeBehavior === "window") toast("仅关闭窗口：任务会在后台继续（真实行为取决于核心进程部署）", "neutral");
    else toast("退出应用：需要先决定如何处理运行中的任务", "blocked");
  };

  A["scan-tools"] = () => {
    st.scanning = true;
    toast("正在扫描本机可执行工具…", "neutral");
    later(1200, () => {
      st.scanning = false;
      toast("扫描完成：1 个可用、1 个待验证、2 个未安装/无法检测", "neutral");
    });
  };

  A["skip-onboarding"] = () => { st.screen = "dashboard"; toast("已跳过引导，可在“本地工具”随时重新扫描", "neutral"); };
  A["finish-onboarding"] = () => { st.screen = "dashboard"; toast("已进入工作台 · 检测结果已保存在“本地工具”", "success"); };

  A["settings-section"] = (el) => { st.settingsSection = el.dataset.section; };
  A["set-retention"] = (el) => { st.logRetention = el.dataset.value; st.menu = null; toast("日志保留已设为 " + el.dataset.value, "neutral"); };

  A["notif-filter"] = (el) => { st.notifFilter = el.dataset.filter; };
  A["mark-all-read"] = () => {
    st.notifications.forEach(n => { n.unread = false; });
    toast("全部通知已标为已读", "success");
  };
  A["open-notification"] = (el) => {
    const n = st.notifications.find(x => x.id === el.dataset.id);
    if (!n) return;
    n.unread = false;
    if (n.taskId) st.taskId = n.taskId;
    st.screen = n.screen || "notifications";
  };
  A["dismiss-notification"] = (el) => {
    st.notifications = st.notifications.filter(n => n.id !== el.dataset.id);
    toast("通知已移除（演示）", "neutral");
  };

  A["dialog-new-workspace"] = () => { st.dialog = "workspace-new"; };
  A["save-workspace"] = () => {
    const name = (document.getElementById("nw-name") || {}).value || "untitled";
    const path = (document.getElementById("nw-path") || {}).value || "~/work/" + name;
    st.workspaces.push({ id: "w" + (st.workspaces.length + 1), name, path, tasks: 0, state: "ready", read: ["docs/"], write: ["docs/"], deny: ["~/.ssh", "~/.config"] });
    st.wsId = st.workspaces[st.workspaces.length - 1].id;
    st.dialog = null;
    toast("工作空间已添加：" + name + "（写入范围默认关闭）", "success");
  };
  A["dialog-ws-boundary"] = () => { st.dialog = "ws-boundary"; };
  A["save-boundary"] = () => {
    const read = (document.getElementById("wb-read") || {}).value || "";
    const write = (document.getElementById("wb-write") || {}).value || "";
    const w = st.workspaces.find(x => x.id === st.wsId);
    if (w) {
      w.read = read.split(",").map(s => s.trim()).filter(Boolean);
      w.write = write.split(",").map(s => s.trim()).filter(Boolean);
    }
    st.dialog = null;
    toast("访问边界已保存（新边界从下一个任务生效）", "success");
  };
  A["resolve-conflict"] = () => {
    const w = st.workspaces.find(x => x.id === st.wsId);
    if (w) { w.state = "ready"; w.conflictInfo = "已排队其中之一（演示）"; }
    toast("已将 Run #1240 排队，等待当前写入完成后开始", "success");
  };

  A["dialog-open-folder"] = () => { st.dialog = "file-path"; };
  A["copy-path"] = (el) => { st.dialog = null; copyText(el.dataset.path, "路径"); };

  A["help-article"] = (el) => { st.helpArticle = el.dataset.article; };
  A["submit-feedback"] = () => {
    const ta = document.getElementById("help-feedback");
    const v = ta ? ta.value.trim() : "";
    if (!v) { toast("请先填写反馈内容", "neutral"); return; }
    st.feedback.push({ text: v, at: new Date().toLocaleTimeString() });
    if (ta) ta.value = "";
    toast("反馈已记录（本机演示，共 " + st.feedback.length + " 条；不会发送到服务器）", "success");
  };

  window.__ACTIONS = A;
  window.__audit = function () {
    const screensList = ["welcome", "dashboard", "tasks", "task", "tools", "roles", "teams", "workspaces", "approvals", "history", "settings", "states", "notifications", "help"];
    const unknown = {};
    const prev = { screen: st.screen, inspectorOpen: st.inspectorOpen };
    for (const sc of screensList) {
      st.screen = sc;
      st.inspectorOpen = sc === "task";
      render();
      document.querySelectorAll("[data-action]").forEach(el => {
        const a = el.dataset.action;
        if (!A[a]) unknown[a] = (unknown[a] || 0) + 1;
      });
    }
    st.screen = prev.screen;
    st.inspectorOpen = prev.inspectorOpen;
    render();
    return unknown;
  };

  /* ---------- wiring ---------- */
  document.addEventListener("click", (e) => {
    const el = e.target.closest("[data-action]");
    if (st.menu && !e.target.closest(".menu-pop") && !e.target.closest("[data-graph-action]") && !(el && /^menu-/.test(el.dataset.action))) {
      st.menu = null;
      render();
      if (!el) return;
    }
    if (!el) return;
    if (el.classList.contains("scrim") && e.target !== el) return;
    if (el.classList.contains("palette") && e.target !== el) return;
    const fn = A[el.dataset.action];
    if (fn) {
      e.preventDefault();
      fn(el, e);
      render();
    }
  });

  document.addEventListener("input", (e) => {
    const t = e.target;
    if (t.id === "composer-input") st.composerText = t.value;
    if (t.id === "log-search") { st.logQuery = t.value; focusAfter = "#log-search"; render(); }
    if (t.id === "task-search") { st.taskQuery = t.value; focusAfter = "#task-search"; render(); }
    if (t.id === "palette-input") { st.paletteQuery = t.value; st.paletteIndex = 0; focusAfter = "#palette-input"; render(); }
    if (t.id === "help-search") { st.helpQuery = t.value; focusAfter = "#help-search"; render(); }
  });

  document.addEventListener("change", (e) => {
    if (e.target.id === "te-ws" || e.target.id === "wb-read") { /* read on save */ }
  });

  document.addEventListener("keydown", (e) => {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "k") {
      e.preventDefault();
      st.palette = !st.palette;
      st.paletteQuery = "";
      st.paletteIndex = 0;
      focusAfter = "#palette-input";
      render();
      return;
    }
    if (st.palette) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        st.paletteIndex += e.key === "ArrowDown" ? 1 : -1;
        const items = DATA.paletteCommands.filter(c => !st.paletteQuery || c.label.toLowerCase().includes(st.paletteQuery.toLowerCase()));
        if (st.paletteIndex < 0) st.paletteIndex = Math.max(0, items.length - 1);
        if (st.paletteIndex >= items.length) st.paletteIndex = 0;
        focusAfter = "#palette-input";
        render();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        A["palette-run"]({ dataset: {} });
        render();
        return;
      }
    }
    if (e.key === "Escape") {
      if (st.menu) { st.menu = null; render(); return; }
      if (st.palette) { st.palette = false; render(); return; }
      if (st.dialog) { st.dialog = null; render(); return; }
      if (st.inspectorOpen) { st.inspectorOpen = false; render(); return; }
    }
    if (e.key === "Enter" && document.activeElement && document.activeElement.id === "task-input" && !e.shiftKey) {
      e.preventDefault();
      A["send-message"]();
      render();
    }
    if (e.key === "Enter" && document.activeElement && document.activeElement.id === "task-search") {
      const first = document.querySelector('tr[data-action="open-task"]');
      if (first) { A["open-task"](first); render(); }
    }
    if (e.key === "Enter" && document.activeElement && document.activeElement.classList.contains("task-row")) {
      const first = document.querySelector(".task-row");
      if (first) { A["open-task"](first); render(); }
    }
  });

  document.addEventListener("DOMContentLoaded", () => {
    window.__ACTIONS = A;
    window.__MENUS = MENUS;
    if (window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches) {
      st.theme = "dark";
      st.themePref = "跟随系统";
    }
    const h = (location.hash || "").replace(/^#\/?/, "");
    if (h && window.SCREENS[h]) st.screen = h;
    window.addEventListener("hashchange", () => {
      const hh = (location.hash || "").replace(/^#\/?/, "");
      if (hh && window.SCREENS[hh] && hh !== st.screen) { st.screen = hh; st.inspectorOpen = false; render(); }
    });
    render();
    setTimeout(() => toast("演示模式：所有执行、CLI 与审批行为均为模拟，不访问真实系统。", "neutral"), 700);
  });
})();
