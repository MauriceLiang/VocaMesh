/* VocaMesh prototype · screens part 1: shell, welcome, dashboard, task detail, all tasks */

(function () {
  const I = UI.icon, B = UI.badge, BT = UI.btn;
  const S = (window.SCREENS = {});

  S.shell = function (st, contentHtml, opts) {
    const o = Object.assign({ page: "", workspace: false, inspector: "" }, opts || {});
    const pendingApprovals = st.approvals.filter(a => a.state === "pending").length;
    const unread = st.notifications.filter(n => n.unread).length;
    const count = { approvals: pendingApprovals, notifications: unread };

    const navItem = (item) => {
      const active = st.screen === item.screen || (item.screen === "tasks" && st.screen === "task");
      const c = count[item.count];
      return `<button class="nav-item ${active ? "is-active" : ""}" data-action="go" data-screen="${item.screen}"
        data-tip="${item.label}" aria-current="${active ? "page" : "false"}">
        ${I(item.icon)}<span class="label">${item.label}</span>
        ${c ? `<span class="nav-count">${c}</span>` : ""}
      </button>`;
    };

    const sidebar = `<aside class="sidebar" aria-label="主导航">
      <div class="brand">
        <span class="mark" aria-hidden="true"></span>
        <span class="txt"><span class="p">VocaMesh</span><span class="b">by Vocafect</span></span>
      </div>
      ${DATA.nav.map(g => `
        <div class="nav-group">${g.group}</div>
        ${g.items.map(navItem).join("")}
      `).join("")}
      <div class="nav-spacer"></div>
      <div class="sidebar-foot">
        ${DATA.footNav.map(navItem).join("")}
      </div>
    </aside>`;

    const toolbar = `<header class="toolbar">
      <span class="page">${o.page}</span>
      <div class="spacer"></div>
      <button class="searchbox" data-action="palette" aria-label="打开命令面板">
        ${I("search", "sm")}<span>搜索任务、Run、Agent</span><span class="spacer"></span><span class="kbd">⌘K</span>
      </button>
      ${UI.iconBtn("bell", "go-notifications", "通知")}
      ${UI.iconBtn(st.theme === "dark" ? "sun" : "moon", "toggle-theme", st.theme === "dark" ? "切换到亮色" : "切换到暗色")}
      ${UI.iconBtn("help", "show-shortcuts", "快捷键与帮助")}
      <span class="avatar" title="本地账户（演示）">M</span>
    </header>`;

    return `<div class="app" data-theme="${st.theme}" data-density="${st.density}" data-platform="${st.platform}" data-motion="${st.motion}">
      <div class="titlebar">
        <div class="lights"><span class="light r"></span><span class="light y"></span><span class="light g"></span></div>
        <span class="title">VocaMesh — ${o.page}</span>
        <span class="demo-tag">mock 演示</span>
        <div class="drag"></div>
        ${UI.winControls()}
      </div>
      <div class="body">
        ${sidebar}
        <div class="main">
          ${toolbar}
          <div class="workspace">
            <div class="content ${o.dense ? "is-task" : ""}" id="content">${contentHtml}</div>
            ${o.inspector || ""}
          </div>
        </div>
      </div>
      ${S.overlays(st)}
    </div>`;
  };

  /* ---------- welcome ---------- */
  S.welcome = function (st) {
    const rows = st.tools.map(t => {
      const map = {
        ready: ["success", "已安装且可用"],
        "auth-unknown": ["blocked", "认证待验证"],
        missing: ["neutral", "未安装"],
        undetectable: ["unknown", "无法检测"]
      };
      const [kind, label] = map[t.state];
      return `<div class="row" style="padding:9px 0;border-top:1px solid var(--border-default)">
        <span class="ib" style="width:26px;height:26px;border-radius:6px;background:var(--bg-subtle);display:grid;place-items:center">${I(t.id === "claude" ? "bot" : t.id === "codex" ? "terminal" : "plug", "sm")}</span>
        <span class="col" style="gap:1px;flex:1 1 auto;min-width:0">
          <span class="row" style="gap:7px"><b style="font-size:12.5px;font-weight:500">${t.name}</b><span class="mono tiny muted">${t.version}</span></span>
          <span class="mono tiny muted">${t.path}</span>
        </span>
        ${B(kind, label)}
      </div>`;
    }).join("");

    const scanning = st.scanning;
    const body = `<div class="col" style="align-items:center;gap:18px;padding:28px 0">
      <div class="row" style="gap:9px">
        <span class="mark" style="width:26px;height:26px;border-radius:7px;background:var(--text-primary)"></span>
        <b style="font-size:16px">VocaMesh</b><span class="tiny muted">by Vocafect</span>
      </div>
      <div class="card" style="width:660px;max-width:100%;padding:28px;display:flex;flex-direction:column;gap:16px">
        <div class="col" style="gap:6px">
          <h1 style="font-size:25px;margin:0;letter-spacing:-0.5px">欢迎使用 VocaMesh</h1>
          <p class="secondary" style="margin:0;font-size:12.5px;line-height:1.65">
            VocaMesh 在本机调用你已安装的 Agent 工具，帮你把一件工作交给多个 Agent 协作完成。
            这里不需要创建组织，也不需要输入任何 API Key。
          </p>
        </div>
        <div class="card" style="overflow:hidden">
          <div class="row between" style="padding:8px 12px;background:var(--bg-inset);font-size:11.5px">
            <b>${scanning ? "正在检查本机可用的执行工具…" : "本机可用的执行工具"}</b>
            <span class="tiny muted">${scanning ? "扫描中" : "上次扫描 3 秒前"}</span>
          </div>
          <div style="padding:0 12px">${rows}</div>
        </div>
        <p class="tiny muted" style="margin:0;line-height:1.6">
          检测只读取可执行文件路径、版本与登录状态提示，不读取任何凭据内容；状态不明时显示“认证待验证”，不会假定可用。
        </p>
        <div class="row" style="gap:8px">
          ${BT({ label: scanning ? "扫描中…" : "重新扫描", variant: "secondary", action: "scan-tools", disabled: scanning })}
          ${BT({ label: "跳过引导，先浏览", variant: "ghost", action: "skip-onboarding" })}
          <span class="spacer"></span>
          ${BT({ label: "开始使用 VocaMesh", variant: "primary", action: "finish-onboarding" })}
        </div>
      </div>
      <p class="tiny muted">下一步可选：创建第一个 Agent 角色（也可稍后在左侧“Agent 角色”中完成）</p>
    </div>`;

    return `<div class="app" data-theme="${st.theme}" data-density="${st.density}" data-platform="${st.platform}" data-motion="${st.motion}">
      <div class="titlebar">
        <div class="lights"><span class="light r"></span><span class="light y"></span><span class="light g"></span></div>
        <span class="title">VocaMesh — 首次启动</span>
        <span class="demo-tag">mock 演示 · 不调用真实 CLI</span>
        <div class="drag"></div>
        ${UI.winControls()}
      </div>
      <div class="content" style="justify-content:center;align-items:center">${body}</div>
      ${S.overlays(st)}
    </div>`;
  };

  /* ---------- dashboard ---------- */
  S.dashboard = function (st) {
    const running = st.tasks.filter(t => ["running", "waiting-approval", "queued", "paused", "needs-attention"].includes(t.state));
    const pending = st.approvals.filter(a => a.state === "pending");

    const metrics = [
      ["执行中", String(st.tasks.filter(t => t.state === "running").length), "loader", "", "running"],
      ["等待你处理", String(pending.length), "shield", "is-blocked", "waiting-approval"],
      ["排队中", String(st.tasks.filter(t => t.state === "queued").length), "hourglass", "", "queued"],
      ["今日已完成", "8", "check", "is-success", "done"]
    ].map(([l, v, ic, cls, state]) => `<button class="metric ${cls}" data-action="metric" data-state="${state}" title="查看${l}的任务">
        <span class="ib">${I(ic, "sm")}</span>
        <span class="col" style="gap:1px;align-items:flex-start"><b class="v">${v}</b><span class="l">${l}</span></span>
      </button>`).join("");

    const taskRows = running.map(t => {
      const [k, label] = UI.taskStateLabel(t.state);
      const done = t.subtasks.filter(s => s.state === "success").length;
      return `<button class="task-row" data-action="open-task" data-id="${t.id}">
        <span class="col" style="gap:3px;flex:1 1 auto;min-width:0">
          <span class="t">${t.name}</span>
          <span class="row" style="gap:8px">
            <span class="m">${t.ws}</span><span class="m">· ${t.team}</span><span class="m">· Run ${t.run}</span>
            <span class="tiny muted">· ${done}/${t.subtasks.length} 子任务</span>
          </span>
        </span>
        ${B(k, label)}
        ${I("right", "sm muted")}
      </button>`;
    }).join("");

    const approvalCards = pending.slice(0, 2).map(a => `
      <div class="card panel" style="gap:7px">
        <div class="row between">${B("blocked", "等待审批")}<span class="tiny muted">${a.age}</span></div>
        <div class="mono" style="font-size:11.5px">${a.title}</div>
        <div class="tiny muted">${a.agent}</div>
        <div class="row tiny" style="gap:5px;color:var(--status-blocked)">${I("alert", "sm")}${a.risk[0]}</div>
        <div class="row" style="justify-content:flex-end;gap:6px">
          ${BT({ label: "拒绝", variant: "ghost", size: "sm", action: "deny-approval", data: { id: a.id } })}
          ${BT({ label: "仅此一次允许", variant: "secondary", size: "sm", action: "allow-approval", data: { id: a.id } })}
        </div>
      </div>`).join("");

    const done = st.tasks.filter(t => t.state === "done").slice(0, 3).map(t => `
      <div class="row" style="gap:9px;padding:8px 0;border-top:1px solid var(--border-default)">
        ${I("check", "sm")}<span style="flex:1 1 auto;font-size:11.5px">${t.name}</span>
        <span class="tiny muted">${t.run}</span>
      </div>`).join("");

    return S.shell(st, `
      <div class="page-head">
        <div>
          <h1 class="page-title">工作台</h1>
          <p class="page-sub">下午好 · ${st.tasks.filter(t => t.state === "running").length} 项任务正在执行，${pending.length} 项等待你处理</p>
        </div>
        <div class="head-actions">
          ${BT({ label: "模板库", variant: "ghost", action: "dialog-templates" })}
          ${BT({ label: "新建任务", variant: "primary", action: "focus-composer" })}
        </div>
      </div>

      <div class="card composer">
        <textarea class="composer-input" id="composer-input" aria-label="描述你要完成的工作"
          placeholder="描述你要完成的工作，例如：把 2025 Q3 三家竞品的公开资料整理成一份对比报告…">${st.composerText || ""}</textarea>
        <div class="row" style="gap:10px;flex-wrap:wrap">
          <button class="chip" data-action="menu-workspace">${I("folder", "sm")}<span class="mono">${st.composerWs}</span>${I("down", "sm")}</button>
          <div class="segmented" role="group" aria-label="执行方式">
            ${["快速执行", "选择团队", "自定义"].map(m => `<button class="${st.composerMode === m ? "is-on" : ""}" data-action="set-mode" data-mode="${m}">${m}</button>`).join("")}
          </div>
          <span class="spacer"></span>
          <span class="tiny muted">${st.composerMode === "快速执行" ? "使用：研究员 · 不额外启动主管" : st.composerMode === "选择团队" ? "使用团队：调研团队 · 主管统一协调" : "高级：子任务、依赖与授权手动定义"}</span>
          ${BT({ label: "创建任务", variant: "primary", action: "create-task" })}
        </div>
      </div>

      <div class="metrics">${metrics}</div>
      ${st.attach.length ? `<div class="row" style="gap:6px;flex-wrap:wrap">
        ${st.attach.map(f => `<span class="chip" style="height:24px;font-size:11px">${I("file", "sm")}${f}<button class="icon-btn" style="width:16px;height:16px" data-action="remove-attach" data-file="${f}" aria-label="移除 ${f}">${I("x", "sm")}</button></span>`).join("")}
      </div>` : ""}

      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="grow col" style="gap:8px;min-width:0">
          <div class="row between"><span class="section-title">进行中的任务</span>
            <button class="btn is-ghost sm" data-action="go" data-screen="tasks">查看全部任务 ${I("right", "sm")}</button></div>
          ${taskRows || `<div class="card empty"><span class="et">开始你的第一项任务</span><span class="small">在上方输入任务后点击“创建任务”。如果还没有可用的执行工具，先去“本地工具”完成安装与认证。</span>
            <div class="row" style="gap:8px">${BT({ label: "去本地工具", variant: "secondary", size: "sm", action: "go", data: { screen: "tools" } })}${BT({ label: "去创建 Agent 角色", variant: "ghost", size: "sm", action: "go", data: { screen: "roles" } })}</div></div>`}
        </div>
        <div class="col col-rail" style="gap:8px">
          <div class="row between"><span class="row" style="gap:7px"><span class="section-title">待审批</span>${pending.length ? `<span class="badge is-blocked" style="padding:0 6px">${pending.length}</span>` : ""}</span>
            <button class="btn is-ghost sm" data-action="go" data-screen="approvals">全部</button></div>
          ${approvalCards || `<div class="card panel"><span class="small muted">没有待处理的审批。</span></div>`}
          <div class="row between" style="margin-top:2px"><span class="section-title">最近完成</span>
            <button class="btn is-ghost sm" data-action="go" data-screen="history">历史记录 ${I("right", "sm")}</button></div>
          <div class="card" style="padding:0 12px 4px">${done}</div>
        </div>
      </div>
    `, { page: "工作台" });
  };

  /* ---------- task detail ---------- */
  S.task = function (st) {
    const t = st.tasks.find(x => x.id === st.taskId) || st.tasks[0];
    const [k, label] = UI.taskStateLabel(t.state);
    const pendingForTask = st.approvals.filter(a => a.taskId === t.id && a.state === "pending");
    const done = t.subtasks.filter(s => s.state === "success").length;
    const runningN = t.subtasks.filter(s => s.state === "running").length;
    const blockedN = t.subtasks.filter(s => ["waiting-approval", "blocked", "waiting-dependency"].includes(s.state)).length;
    const waitN = t.subtasks.filter(s => s.state === "pending").length;
    const failN = t.subtasks.filter(s => s.state === "failure").length;

    const actions = (() => {
      const a = [];
      if (t.state === "running") {
        a.push(BT({ label: "暂停", variant: "secondary", action: "pause-task", icon: "pause" }));
        a.push(BT({ label: "停止", variant: "secondary", action: "stop-task", icon: "square" }));
      } else if (t.state === "pause-pending") {
        a.push(BT({ label: "等待暂停…", variant: "secondary", disabled: true, icon: "loader" }));
        a.push(BT({ label: "请求停止当前执行", variant: "secondary", action: "stop-task" }));
      } else if (t.state === "paused") {
        a.push(BT({ label: "继续", variant: "primary", action: "resume-task", icon: "play" }));
        a.push(BT({ label: "停止", variant: "secondary", action: "stop-task" }));
      } else if (t.state === "stop-pending") {
        a.push(BT({ label: "正在请求停止…", variant: "secondary", disabled: true }));
        a.push(BT({ label: "查看详情", variant: "ghost", action: "inspector-open", data: { tab: "activity" } }));
      } else if (t.state === "waiting-approval") {
        a.push(BT({ label: "去处理审批", variant: "primary", action: "go", data: { screen: "approvals" } }));
        a.push(BT({ label: "暂停", variant: "secondary", action: "pause-task" }));
      } else if (t.state === "needs-attention" || t.state === "partial" || t.state === "failed") {
        a.push(BT({ label: "仅重试失败部分", variant: "primary", action: "retry-failed" }));
        a.push(BT({ label: "跳过并生成部分成果", variant: "secondary", action: "skip-partial" }));
        a.push(BT({ label: "停止整个任务", variant: "ghost", action: "stop-task" }));
      } else if (t.state === "stopped") {
        a.push(BT({ label: "在新 Run 中重新开始", variant: "primary", action: "retry-failed" }));
      }
      a.push(UI.iconBtn("panel", st.inspectorOpen ? "inspector-close" : "inspector-open", "检查器"));
      return a.join("");
    })();

    const banner = (() => {
      if (pendingForTask.length) {
        return `<div class="banner is-blocked">${I("alert", "sm")}
          <span style="flex:1 1 auto">${pendingForTask.length} 项子任务等待你的审批：${pendingForTask[0].title.replace("写入文件 ", "")}。在决定前，本任务不会继续派发新工作。</span>
          ${BT({ label: "查看审批", variant: "secondary", size: "sm", action: "go", data: { screen: "approvals" } })}
        </div>`;
      }
      if (t.state === "pause-pending") {
        return `<div class="banner is-neutral">${I("pause", "sm")}
          <span style="flex:1 1 auto">等待暂停：已停止派发新工作，当前执行仍在运行。到达安全边界后才会显示“已暂停”。</span>
          ${BT({ label: "请求停止当前执行", variant: "secondary", size: "sm", action: "stop-task" })}</div>`;
      }
      if (t.state === "paused") {
        return `<div class="banner is-neutral">${I("pause", "sm")}
          <span style="flex:1 1 auto">已暂停：所有执行均已到达安全边界，调度停止。继续前会检查权限、依赖与输入版本。</span>
          ${BT({ label: "继续", variant: "secondary", size: "sm", action: "resume-task" })}</div>`;
      }
      if (t.state === "stop-pending") {
        return `<div class="banner is-neutral">${I("loader", "sm")}
          <span style="flex:1 1 auto">正在请求停止：已向 2 个运行时发出停止请求，尚未全部确认。这不等于“已停止”。</span>
          ${BT({ label: "状态待确认？", variant: "ghost", size: "sm", action: "inspector-open", data: { tab: "activity" } })}</div>`;
      }
      if (t.state === "stopped") {
        return `<div class="banner is-neutral">${I("square", "sm")}
          <span style="flex:1 1 auto">已停止：${done} 项成果已保留，${t.subtasks.length - done} 项未完成。重试会创建新的 Run，旧记录不变。</span></div>`;
      }
      if (t.state === "needs-attention" || t.state === "failed" || t.state === "partial") {
        return `<div class="banner is-failure">${I("x", "sm")}
          <span style="flex:1 1 auto">${t.state === "failed" ? "执行失败" : "需要处理"}：${t.stateNote}。无关子任务已完成，依赖失败节点的子任务已被阻塞。</span>
          ${BT({ label: "仅重试失败部分", variant: "secondary", size: "sm", action: "retry-failed" })}</div>`;
      }
      if (t.state === "queued") {
        return `<div class="banner is-neutral">${I("hourglass", "sm")}
          <span style="flex:1 1 auto">排队等待资源：当前并发 3/3。任务未被丢弃，资源释放后自动开始；也可调整并发上限。</span>
          ${BT({ label: "调整并发", variant: "secondary", size: "sm", action: "go", data: { screen: "settings" } })}</div>`;
      }
      return "";
    })();

    const flow = t.subtasks.map((s, i) => {
      const [sk, sl] = UI.subStateLabel(s.state);
      const iconName = s.state === "success" ? "check" : s.state === "failure" ? "x" : s.state === "waiting-approval" ? "shield" : s.state === "running" ? "loader" : s.state === "waiting-dependency" ? "hourglass" : s.state === "blocked" ? "lock" : "dot";
      return `<button class="flow-node ${s.state === "waiting-approval" || s.state === "blocked" ? "is-blocked" : ""} ${s.state === "failure" ? "is-failure" : ""} ${s.state === "success" ? "is-success" : ""} ${st.highlight === s.id ? "is-hl" : ""}"
        data-action="open-subtask" data-id="${s.id}">
        <span class="nb">${I(iconName, "sm")}</span>
        <span class="col" style="gap:2px;flex:1 1 auto;min-width:0">
          <span class="nt">${s.name}</span>
          <span class="row" style="gap:7px"><span class="nm">${s.agent}</span><span class="nm">· ${s.result}</span></span>
        </span>
        ${B(sk, sl)}
        ${I("right", "sm muted")}
      </button>`;
    }).join("");

    const approvalCard = (() => {
      const a = pendingForTask[0];
      if (!a) return "";
      return `<div class="event-card">
        <span class="col" style="gap:5px;flex:1 1 auto;min-width:0">
          <span class="row" style="gap:8px">${B("blocked", "等待审批")}<span class="tiny secondary">写作者请求写入文件</span></span>
          <span class="mono" style="font-size:11px">${a.target}</span>
          <span class="tiny muted">${a.scope}</span>
        </span>
        <span class="row" style="gap:6px;align-items:center;flex:0 0 auto">
          ${BT({ label: "拒绝", variant: "ghost", size: "sm", action: "deny-approval", data: { id: a.id } })}
          ${BT({ label: "仅此一次允许", variant: "primary", size: "sm", action: "allow-approval", data: { id: a.id } })}
        </span>
      </div>`;
    })();

    const instr = t.instruction ? (() => {
      const map = {
        queued: ["neutral", "排队待投递"],
        delivered: ["neutral", "已送达（运行时确认）"],
        applied: ["success", "已应用"],
        "not-applied": ["failure", "未应用"],
        "waiting-point": ["blocked", "等待安全投递点"],
        failed: ["failure", "投递失败"]
      };
      const [ik, il] = map[t.instruction.state] || ["neutral", t.instruction.state];
      return `<div class="card panel" style="gap:6px;background:var(--bg-inset)">
        <div class="row between">
          <span class="row" style="gap:6px">${I("send", "sm")}<b class="small">定向指令 · 目标 ${t.instruction.target}</b></span>
          ${B(ik, il)}
        </div>
        <div class="small secondary">${t.instruction.text}</div>
        <div class="tiny muted">已同步主管（结构化事件，不代表主管实时读取全部日志）· ${t.instruction.impact || "未检测到计划变更"}</div>
      </div>`;
    })() : "";

    const messages = t.messages.map(m => m.who === "me"
      ? `<div class="msg me"><div class="bubble">${m.text}<div class="tiny muted" style="margin-top:4px">你 · ${m.time}</div></div></div>`
      : `<div class="msg"><span class="avatar-sq">主</span><div class="bubble"><div><span class="who">${m.name}</span><span class="time">${m.time}</span></div><div>${m.text}</div></div></div>`
    ).join("");

    const content = `
      <div class="task-scroll">
      <div class="task-head">
        <div class="row" style="gap:8px;flex-wrap:wrap">
          ${UI.iconBtn("left", "go", "返回全部任务", 'data-screen="tasks"')}
          <h1 class="task-title" style="flex:1 1 auto;min-width:0">${t.name}</h1>
          ${B(k, label)}
          <span class="spacer"></span>
          ${actions}
          ${UI.iconBtn("more", "menu-task-more", "更多任务操作")}
        </div>
        <div class="meta-line">
          <span>${I("folder", "sm")}${t.ws}</span>
          <span>${I("users", "sm")}${t.team}</span>
          <span>${I("branch", "sm")}${t.run}</span>
          <span>${I("hourglass", "sm")}开始于 ${t.started} · ${t.stateNote}</span>
        </div>
      </div>
      ${banner}
      <div class="summary-strip">
        <div class="stat"><b style="color:var(--status-success)">${done}</b><span>已完成</span></div>
        <div class="stat"><b>${runningN}</b><span>执行中</span></div>
        <div class="stat"><b style="color:var(--status-blocked)">${blockedN}</b><span>阻塞/待审批</span></div>
        <div class="stat"><b>${failN || waitN}</b><span>${failN ? "失败" : "未开始"}</span></div>
        <div class="stat"><b>${t.subtasks.length}</b><span>子任务总数</span></div>
        <span class="spacer"></span>
        <div class="segmented" style="margin-right:8px">
          <button class="${st.taskFocus === "flow" ? "is-on" : ""}" data-action="task-focus" data-focus="flow">流程聚焦</button>
          <button class="${st.taskFocus === "chat" ? "is-on" : ""}" data-action="task-focus" data-focus="chat">对话聚焦</button>
        </div>
      </div>

      <div class="row between"><span class="section-title">执行流程</span>
        <span class="tiny muted">组织关系 ≠ 任务依赖：这里表达先后依赖</span></div>
      <div class="flow">${flow}</div>

      <div class="row between" style="margin-top:2px"><span class="section-title">主管对话</span>
        <span class="tiny muted">原始 Agent 日志在右侧检查器中按需查看</span></div>
      <div class="chat" style="min-height:200px">${approvalCard}${instr}${messages}</div>
      </div>

      <div class="task-composer">
        ${st.attach.length ? `<div class="row" style="gap:6px;flex-wrap:wrap">${st.attach.map(f => `<span class="chip" style="height:22px;font-size:10.5px">${I("file", "sm")}${f}<button class="icon-btn" style="width:14px;height:14px" data-action="remove-attach" data-file="${f}" aria-label="移除 ${f}">${I("x", "sm")}</button></span>`).join("")}</div>` : ""}
        <div class="row" style="gap:8px">
          <textarea id="task-input" placeholder="向主管提问，或输入 @ 指定某个 Agent 发送定向指令…" aria-label="向主管发送消息"></textarea>
        </div>
        <div class="foot">
          <span>发送给：主管（${t.supervisor}）</span>
          <span>· 定向指令会同步主管，并显示投递状态</span>
          <span class="spacer"></span>
          <button class="chip" data-action="toggle-instruction-target" title="切换发送对象">@ ${st.instrTarget || "主管"} ${I("down", "sm")}</button>
          ${UI.iconBtn("clip", "menu-attach", "添加附件")}
          ${BT({ label: "发送", size: "sm", variant: "secondary", action: "send-message", icon: "up" })}
        </div>
      </div>
    `;

    return S.shell(st, content, { page: "全部任务 · 任务详情", inspector: S.inspector(st, t), dense: true });
  };

  /* ---------- inspector ---------- */
  S.inspector = function (st, t) {
    if (!st.inspectorOpen) return "";
    const tab = st.inspectorTab;
    const sub = t.subtasks.find(s => s.id === st.selectedSubtask) || t.subtasks[2] || t.subtasks[0];
    const tabs = ["activity", "logs", "results", "info"].map(x => {
      const label = { activity: "活动", logs: "日志", results: "成果", info: "信息" }[x];
      return `<button class="${tab === x ? "is-on" : ""}" data-action="inspector-tab" data-tab="${x}">${label}</button>`;
    }).join("");

    let body = "";
    if (tab === "activity") {
      body = `<div class="col" style="padding:12px 14px;gap:9px;overflow:auto">
        ${t.subtasks.map(s => {
          const [sk, sl] = UI.subStateLabel(s.state);
          return `<div class="row" style="gap:8px;align-items:flex-start">
            <span class="mono tiny muted" style="width:44px">${s.id.toUpperCase()}</span>
            <span class="col" style="gap:1px;flex:1 1 auto"><span class="small">${s.name}</span><span class="tiny muted">${s.agent} · ${s.result}</span></span>
            ${B(sk, sl)}
          </div>`;
        }).join('<div class="divider"></div>')}
        <div class="divider"></div>
        <div class="row" style="gap:8px;align-items:flex-start"><span class="mono tiny muted" style="width:44px">EVT</span>
          <span class="col" style="gap:1px"><span class="small">审批请求已发出</span><span class="tiny muted">14:04:02 · 已送达运行时 · 等待用户决定</span></span></div>
      </div>`;
    } else if (tab === "logs") {
      const q = (st.logQuery || "").toLowerCase();
      const lines0 = DATA.logs.filter(l => !q || l.join(" ").toLowerCase().includes(q));
      const extra = Array.from({ length: st.logsMore }, (_, i) => ["14:0" + (1 + Math.floor(i / 6)) + ":" + String(10 + i).padStart(2, "0"), "DEBUG", "adapter", "earlier heartbeat · seq=" + (231 - i)]);
      const lines = extra.concat(lines0);
      body = `<div class="log-controls">
          <div class="row" style="gap:8px">
            <label class="searchbox" style="flex:1 1 auto;min-width:0">${I("search", "sm")}
              <input id="log-search" value="${UI.esc(st.logQuery || "")}" placeholder="筛选日志关键字" aria-label="筛选日志关键字"
                style="border:0;background:transparent;outline:none;font-size:11.5px;width:100%" />
            </label>
            <button class="row" style="gap:6px" data-action="toggle-autoscroll" aria-pressed="${st.autoscroll}">
              <span class="switch ${st.autoscroll ? "is-on" : ""}"></span><span class="tiny secondary">自动滚动</span>
            </button>
          </div>
          <div class="row" style="gap:6px">
            ${["全部", "警告", "错误"].map(f => `<button class="chip ${st.logFilter === f ? "is-on" : ""}" data-action="log-filter" data-filter="${f}" style="height:22px;font-size:10.5px">${f}</button>`).join("")}
            <span class="spacer"></span>
            <span class="tiny muted">${lines.length} 行 · 演示数据</span>
          </div>
        </div>
        <div class="divider"></div>
        <div class="log-body" id="log-body">
          ${lines.map(([t2, lv, who, msg]) => {
            if (st.logFilter === "警告" && lv !== "WARN") return "";
            if (st.logFilter === "错误" && lv !== "ERROR") return "";
            return `<div class="log-line ${lv === "WARN" ? "warn" : ""}">
              <span class="muted">${t2}</span><span class="lv">${lv}</span><span class="muted">${who}</span><span class="msg">${UI.esc(msg)}</span>
            </div>`;
          }).join("")}
        </div>
        <div class="log-foot">
          <span class="tiny muted" style="flex:1 1 auto">显示 ${lines.length} / 231 行</span>
          <button class="btn is-ghost sm" data-action="copy-logs">${I("copy", "sm")}复制</button>
          <button class="btn is-ghost sm" data-action="logs-more">加载更早</button>
        </div>`;
    } else if (tab === "results") {
      body = `<div class="col" style="padding:12px 14px;gap:9px;overflow:auto">
        ${[["docs/compare-draft.md", "进行中 · 2.4 KB"], ["exports/sources.csv", "已完成 · 12 行"], ["docs/report-q3.md", "等待授权写入"]].map(([f, s2]) => `
          <div class="row card" style="padding:8px 10px;gap:8px">${I("file", "sm")}
            <span class="col" style="gap:1px;flex:1 1 auto"><span class="mono" style="font-size:11px">${f}</span><span class="tiny muted">${s2}</span></span>
            ${BT({ label: "预览", variant: "ghost", size: "sm", action: "file-preview", data: { path: f } })}
          </div>`).join("")}
        <div class="tiny muted">路径为模拟路径，不会访问真实文件。</div>
      </div>`;
    } else {
      body = `<div class="col" style="padding:12px 14px;gap:8px;overflow:auto">
        ${[["Agent 角色", sub.agent], ["运行工具", "Claude Code · v2.1.4"], ["工作目录", "~/work/" + t.ws], ["权限边界", "读: " + t.ws + "/  写: exports/  禁止: ~/.ssh"], ["关联 Run", t.run], ["状态", t.stateNote]]
          .map(([k2, v2]) => `<div class="kv"><span class="k">${k2}</span><span style="flex:1 1 auto">${v2}</span></div>`).join("")}
        <div class="divider"></div>
        <div class="row" style="gap:8px">${BT({ label: "查看日志", variant: "secondary", size: "sm", action: "inspector-tab", data: { tab: "logs" } })}
          ${BT({ label: "在时间线定位", variant: "ghost", size: "sm", action: "locate-subtask" })}</div>
      </div>`;
    }

    return `<aside class="inspector" aria-label="检查器">
      <div class="inspector-head">
        <span class="col" style="gap:1px;flex:1 1 auto;min-width:0">
          <b class="small">${sub.name}</b>
          <span class="tiny muted">${sub.agent}</span>
        </span>
        ${UI.iconBtn("x", "inspector-close", "关闭检查器")}
      </div>
      <div class="tabs">${tabs}</div>
      ${body}
    </aside>`;
  };

  /* ---------- all tasks ---------- */
  S.tasks = function (st) {
    const q = (st.taskQuery || "").toLowerCase();
    const f = st.filters;
    const stateMap = { "执行中": ["running"], "等待审批": ["waiting-approval"], "排队中": ["queued"], "已暂停": ["paused", "pause-pending"], "需要处理": ["needs-attention", "partial", "failed"], "已完成": ["done"], "失败": ["failed"] };
    const rows = st.tasks.filter(t => {
      if (q && !(t.name + t.ws + t.run).toLowerCase().includes(q)) return false;
      if (f.taskState !== "全部" && !(stateMap[f.taskState] || [f.taskState]).includes(t.state)) return false;
      if (f.taskWs !== "全部" && t.ws !== f.taskWs) return false;
      if (f.taskWho !== "全部" && t.team.indexOf(f.taskWho.replace("团队", "")) < 0) return false;
      return true;
    });
    const body = rows.map(t => {
      const [k, label] = UI.taskStateLabel(t.state);
      const done = t.subtasks.filter(s => s.state === "success").length;
      return `<tr class="clickable" data-action="open-task" data-id="${t.id}" tabindex="0">
        <td>${B(k, label)}</td>
        <td><div style="font-weight:500">${t.name}</div><div class="tiny muted">${done}/${t.subtasks.length} 子任务 · ${t.stateNote}</div></td>
        <td class="mono tiny secondary">${t.ws}</td>
        <td class="small secondary">${t.team}</td>
        <td class="mono tiny muted">${t.run}</td>
        <td class="small muted">${t.started}</td>
        <td><button class="icon-btn" data-action="menu-row-task" data-id="${t.id}" aria-label="任务操作" title="任务操作">${I("more", "sm")}</button></td>
      </tr>`;
    }).join("");
    const chips = [["状态", f.taskState, "task-state"], ["工作空间", f.taskWs, "task-ws"], ["执行者", f.taskWho, "task-who"], ["时间", f.taskTime, "task-time"]]
      .map(([label, value, kind]) => BT({ label: label + "：" + value, variant: "secondary", size: "sm", action: "menu-filter", data: { kind }, icon: "down" })).join("");

    return S.shell(st, `
      <div class="page-head">
        <div>
          <h1 class="page-title">全部任务</h1>
          <p class="page-sub">${st.tasks.length} 个任务 · ${st.tasks.filter(t => t.state === "running").length} 个执行中 · ${st.approvals.filter(a => a.state === "pending").length} 个等待你处理　（当前工作在此管理；历史记录用于追溯已结束的 Run）</p>
        </div>
        <div class="head-actions">
          ${BT({ label: "筛选：" + f.taskState, variant: "secondary", action: "menu-filter", data: { kind: "task-state" }, icon: "sliders" })}
          ${BT({ label: "新建任务", variant: "primary", action: "dialog-create-task" })}
        </div>
      </div>
      <div class="row" style="gap:8px;flex-wrap:wrap">
        <label class="searchbox" style="min-width:260px">${I("search", "sm")}
          <input id="task-search" value="${UI.esc(st.taskQuery || "")}" placeholder="搜索任务名称、工作空间或 Run" aria-label="搜索任务"
            style="border:0;background:transparent;outline:none;font-size:12px;width:100%" /></label>
        ${chips}
        <span class="spacer"></span>
        <button class="btn is-ghost sm" data-action="clear-task-filter">清除筛选</button>
      </div>
      <div class="table-wrap" style="flex:1 1 auto;overflow:auto">
        <table class="table">
          <thead><tr><th>状态</th><th>任务</th><th>工作空间</th><th>执行者</th><th>Run</th><th>更新</th><th></th></tr></thead>
          <tbody>${body || `<tr><td colspan="7"><div class="empty"><span class="et">没有匹配的任务</span><span class="small">试试清除筛选，或创建第一项任务。</span></div></td></tr>`}</tbody>
        </table>
      </div>
      <div class="progress-note">演示数据：所有任务、Run 与路径均为虚构。点击行进入任务详情；审批入口仍以“待审批”为准，避免双份记录。</div>
    `, { page: "全部任务" });
  };
})();
