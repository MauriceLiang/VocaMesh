/* VocaMesh prototype · screens part 2: tools, roles, teams, workspaces, approvals, history,
   settings, state gallery, notifications, help, overlays (menus / dialogs / palette / toast) */

(function () {
  const I = UI.icon, B = UI.badge, BT = UI.btn;
  const S = window.SCREENS;
  const esc = UI.esc;

  const TOOL_STATE = {
    ready: ["success", "已安装且可用"],
    "auth-unknown": ["blocked", "认证待验证"],
    missing: ["neutral", "未安装"],
    undetectable: ["unknown", "无法检测"],
    incompatible: ["failure", "不兼容"]
  };
  const AGENT_STATE = {
    ready: ["success", "可用"],
    "auth-unknown": ["blocked", "需登录"],
    "not-ready": ["unknown", "未就绪"],
    unknown: ["neutral", "待验证"]
  };
  const RUN_STATE = {
    running: ["running", "执行中"],
    success: ["success", "已完成"],
    failure: ["failure", "异常退出"],
    partial: ["blocked", "部分完成"],
    stopped: ["neutral", "已停止"],
    unknown: ["unknown", "状态未知"]
  };

  /* ---------- tools ---------- */
  S.tools = function (st) {
    const counts = {
      ready: st.tools.filter(t => t.state === "ready").length,
      "auth-unknown": st.tools.filter(t => t.state === "auth-unknown").length,
      missing: st.tools.filter(t => t.state === "missing").length,
      undetectable: st.tools.filter(t => t.state === "undetectable").length
    };
    const summary = `<div class="metrics">
      ${[['可用', counts.ready, 'var(--status-success)', 'check'], ['认证待验证', counts['auth-unknown'], 'var(--status-blocked)', 'alert'], ['未安装', counts.missing, 'var(--text-secondary)', 'plug'], ['无法检测', counts.undetectable, 'var(--text-muted)', 'help']]
        .map(([l, v, c, ic]) => `<div class="metric"><span class="ib" style="color:${c}">${I(ic, 'sm')}</span>
          <span class="col" style="gap:1px"><b class="v">${v}</b><span class="l">${l}</span></span></div>`).join("")}
    </div>`;

    const cards = st.tools.map(t => {
      const [k, label] = TOOL_STATE[t.state];
      const isDefault = st.defaultTool === t.id;
      const actions = t.state === "ready"
        ? BT({ label: "验证登录", variant: "secondary", size: "sm", action: "verify-tool", data: { id: t.id } }) +
          BT({ label: isDefault ? "默认工具 ✓" : "设为默认", variant: "ghost", size: "sm", action: "set-default-tool", data: { id: t.id } }) +
          BT({ label: "查看配置", variant: "ghost", size: "sm", action: "dialog-tool-config", data: { id: t.id } })
        : t.state === "auth-unknown"
        ? BT({ label: "测试认证", variant: "primary", size: "sm", action: "verify-tool", data: { id: t.id } }) +
          BT({ label: "登录指南", variant: "ghost", size: "sm", action: "dialog-install-guide", data: { id: t.id } }) +
          BT({ label: "手动指定路径", variant: "ghost", size: "sm", action: "dialog-tool-path", data: { id: t.id } })
        : t.state === "missing"
        ? BT({ label: "安装指南", variant: "secondary", size: "sm", action: "dialog-install-guide", data: { id: t.id } }) +
          BT({ label: "手动指定路径", variant: "ghost", size: "sm", action: "dialog-tool-path", data: { id: t.id } })
        : BT({ label: "重新检测", variant: "secondary", size: "sm", action: "verify-tool", data: { id: t.id } }) +
          BT({ label: "查看错误详情", variant: "ghost", size: "sm", action: "dialog-tool-error", data: { id: t.id } }) +
          BT({ label: "手动指定路径", variant: "ghost", size: "sm", action: "dialog-tool-path", data: { id: t.id } });

      return `<div class="card panel" style="gap:9px">
        <div class="row" style="gap:10px">
          <span class="ib" style="width:30px;height:30px;border-radius:7px;background:var(--bg-subtle);display:grid;place-items:center">${I(t.id === "claude" ? "bot" : t.id === "codex" ? "terminal" : "plug", "sm")}</span>
          <span class="col" style="gap:1px;flex:1 1 auto;min-width:0">
            <span class="row" style="gap:7px"><b style="font-size:13px">${t.name}</b><span class="mono tiny muted">${t.version}</span>
              ${isDefault ? `<span class="badge is-neutral">默认</span>` : ""}</span>
            <span class="mono tiny muted">适配器 ${t.adapter} · ${t.path}</span>
          </span>
          ${B(k, label)}
        </div>
        <div class="banner ${t.state === "auth-unknown" ? "is-blocked" : "is-neutral"}" style="padding:6px 9px">
          ${I(t.state === "ready" ? "shieldcheck" : "info", "sm")}<span style="flex:1 1 auto">${t.note}</span>
        </div>
        <div class="row" style="gap:7px;flex-wrap:wrap">${actions}</div>
      </div>`;
    }).join("");

    const guide = `<div class="card panel col-rail" style="gap:10px">
      <span class="section-title">安装与认证说明</span>
      <p class="tiny secondary" style="margin:0;line-height:1.55">VocaMesh 不要求也不保存 API Key。是否需要登录由各个 CLI 自己管理，我们只读取它们报告的登录状态。</p>
      ${[["1", "安装 CLI", "在官方渠道安装后，回到本页点击“重新扫描”。"],
         ["2", "完成 CLI 登录", "在终端中运行该 CLI 的登录命令，不经过 VocaMesh。"],
         ["3", "验证状态", "点击“测试认证”，确认可用后再创建 Agent 角色。"]]
        .map(([n, t, d]) => `<div class="row" style="gap:9px;align-items:flex-start">
          <span class="ib" style="width:18px;height:18px;border-radius:50%;background:var(--bg-subtle);display:grid;place-items:center;font-size:10px;font-weight:600">${n}</span>
          <span class="col" style="gap:1px"><b class="small">${t}</b><span class="tiny muted" style="line-height:1.5">${d}</span></span>
        </div>`).join("")}
      <div class="banner is-neutral" style="padding:8px">${I("lock", "sm")}<span class="tiny">检测失败不等于不可用：状态会显示“无法检测”，不会假定成功，也不会阻止你继续浏览。</span></div>
      <span class="spacer"></span>
      <div class="divider"></div>
      <div class="row" style="gap:8px;align-items:center">
        <span class="tiny muted" style="flex:1 1 auto">需要更详细的步骤？</span>
        ${BT({ label: "打开帮助", variant: "secondary", size: "sm", action: "go", data: { screen: "help" } })}
      </div>
    </div>`;

    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">本地工具</h1>
          <p class="page-sub">这台电脑上有哪些可以运行 Agent 的执行环境（CLI 或适配器）。工具是运行环境，不是 Agent 身份。</p></div>
        <div class="head-actions">
          ${BT({ label: st.scanning ? "扫描中…" : "重新扫描", variant: "secondary", action: "scan-tools", icon: "refresh", disabled: st.scanning })}
          ${BT({ label: "手动指定路径", variant: "primary", action: "dialog-tool-path", data: { id: "opencode" }, icon: "plus" })}
        </div>
      </div>
      ${summary}
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="grow col" style="gap:8px;min-width:0;overflow:auto">${cards}</div>
        ${guide}
      </div>
    `, { page: "本地工具" });
  };

  /* ---------- roles ---------- */
  S.roles = function (st) {
    const list = st.agents.map(a => {
      const [k, label] = AGENT_STATE[a.state];
      const on = a.id === st.agentId;
      return `<button class="card row" style="padding:12px;gap:10px;text-align:left;${on ? "border:2px solid var(--text-primary);padding:11px" : ""}" data-action="select-agent" data-id="${a.id}">
        <span class="ib" style="width:28px;height:28px;border-radius:7px;background:var(--bg-subtle);display:grid;place-items:center;font-size:12px;font-weight:600;color:var(--text-secondary)">${a.name.slice(0, 1)}</span>
        <span class="col" style="gap:1px;flex:1 1 auto;min-width:0">
          <b style="font-size:12.5px;font-weight:500">${esc(a.name)}</b><span class="tiny muted">${esc(a.role)}</span>
        </span>
        ${B(k, label)}
      </button>`;
    }).join("");

    const a = st.agents.find(x => x.id === st.agentId) || st.agents[0];
    if (!a) return S.shell(st, `<div class="empty"><span class="et">还没有 Agent 角色</span><span class="small">创建第一个角色后即可分配给任务。</span></div>`, { page: "Agent 角色" });
    const [ak, al] = AGENT_STATE[a.state];
    const permRows = [["read", "读取工作空间文件", "只能读取 " + st.composerWs + " 及子目录"],
      ["write", "写入与修改文件", "写入前对高风险文件请求授权"],
      ["shell", "执行 shell 命令", "白名单命令可直接执行"],
      ["net", "访问网络", "仅研究类角色可开启，限 20 次/任务"]]
      .map(([key, name, desc]) => `<div class="row" style="padding:7px 10px;gap:9px;border-top:1px solid var(--border-default)">
        <span class="col" style="gap:1px;flex:1 1 auto"><span class="small">${name}</span><span class="tiny muted">${desc}</span></span>
        <button class="switch ${a.perms[key] ? "is-on" : ""}" data-action="toggle-perm" data-key="${key}" role="switch" aria-checked="${a.perms[key]}" aria-label="${name}"></button>
      </div>`).join("");

    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">Agent 角色</h1>
          <p class="page-sub">决定一个执行环境扮演什么职责。同一个工具可以支持多个角色，无需重复安装。</p></div>
        <div class="head-actions">${BT({ label: "新建 Agent", variant: "primary", action: "dialog-new-role", icon: "plus" })}</div>
      </div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="col col-list" style="gap:8px;overflow:auto">${list}</div>
        <div class="grow card detail-panel">
          <div class="row" style="gap:8px">
            <span class="section-title" style="font-size:15px">编辑 Agent 角色</span>${B(ak, al)}
            <span class="spacer"></span>
            ${BT({ label: "删除角色", variant: "ghost", size: "sm", action: "confirm-delete-role", data: { id: a.id } })}
            ${BT({ label: "保存修改", variant: "primary", size: "sm", action: "save-role" })}
          </div>
          <div class="grid-2">
            <div class="field"><label class="lbl" for="role-name">名称</label>
              <input class="input" id="role-name" value="${esc(a.name)}" /></div>
            <div class="field"><span class="lbl">执行工具</span>
              <button class="select" data-action="menu-role-tool"><span>${esc(a.tool)}</span>${I("down", "sm")}</button></div>
          </div>
          <div class="field"><label class="lbl" for="role-desc">角色描述</label>
            <textarea class="textarea" id="role-desc" style="min-height:58px">${esc(a.desc)}</textarea></div>
          <div class="field">
            <div class="row" style="gap:6px"><button class="row" style="gap:6px" data-action="toggle-advanced-role">${I(st.roleAdvanced ? "down" : "right", "sm")}<span class="lbl">角色指令（高级）</span></button>
              <span class="tiny muted">${st.roleAdvanced ? "已展开" : "默认收起"}</span></div>
            ${st.roleAdvanced ? `<div class="code-block" style="white-space:pre-wrap">${esc(a.instr)}</div>` : ""}
          </div>
          <div class="field"><span class="lbl">权限范围</span>
            <div class="card" style="overflow:hidden">${permRows}</div></div>
          <div class="field">
            <button class="row" style="gap:8px;padding:9px 10px;border:1px solid var(--border-default);border-radius:6px;width:100%;text-align:left" data-action="toggle-role-exec" aria-expanded="${st.roleExec}">
              ${I(st.roleExec ? "down" : "right", "sm muted")}<span class="small secondary" style="flex:1 1 auto">执行参数：并发、超时、重试策略、模型参数</span><span class="tiny muted">${st.roleExec ? "已展开" : "默认收起"}</span></button>
            ${st.roleExec ? `<div class="card" style="padding:10px 12px;display:flex;flex-direction:column;gap:7px;margin-top:6px">
              ${[["最大并发", "2 个 Run"], ["单步超时", "10 分钟（超时 → 状态待确认，不自动重试）"], ["重试策略", "默认不自动重试；仅启动失败且无副作用时可重试一次"], ["模型参数", "由 CLI 管理，VocaMesh 不覆盖"]]
                .map(([k2, v2]) => `<div class="kv"><span class="k">${k2}</span><span style="flex:1 1 auto">${v2}</span></div>`).join("")}
            </div>` : ""}
          </div>
          <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}
            <span class="tiny">保存只代表配置已保存；正在运行的 Run 不会静默采用新配置，改动从下一个新任务/新 Run 开始生效。</span></div>
        </div>
      </div>
    `, { page: "Agent 角色" });
  };

  /* ---------- org chart (关系图) ---------- */
  S.orgChart = function (st, team) {
    const positions = window.GRAPH.positions(team, st);
    const sup = team.supervisor;
    const edgeOf = (m) => {
      if (m.name === sup) return null;
      const parent = team.members.find(x => x.name === m.reports);
      return parent ? parent.name : (team.members.find(x => x.name === sup) ? sup : null);
    };
    const edges = team.members.map(m => {
      const from = edgeOf(m);
      if (!from || !positions[from] || !positions[m.name]) return "";
      const a = positions[from], b = positions[m.name];
      const x1 = a.x + 98, y1 = a.y + 54, x2 = b.x + 98, y2 = b.y;
      const mid = Math.max(24, (y2 - y1) / 2);
      const hl = st.graphSel === m.name || st.graphSel === from;
      return `<path data-child="${m.name}" data-parent="${from}" class="${hl ? "is-hl" : ""}" d="M${x1} ${y1} C${x1} ${y1 + mid} ${x2} ${y2 - mid} ${x2} ${y2}" />`;
    }).join("");

    const maxX = Math.max.apply(null, team.members.map(m => (positions[m.name] || { x: 0 }).x)) + 236;
    const maxY = Math.max.apply(null, team.members.map(m => (positions[m.name] || { y: 0 }).y)) + 120;

    const nodes = team.members.map(m => {
      const p = positions[m.name] || { x: 0, y: 0 };
      const stMap = AGENT_STATE[m.state] || ["neutral", "待验证"];
      const dot = stMap[0] === "success" ? "is-success" : stMap[0] === "blocked" ? "is-blocked" : "is-unknown";
      const isRoot = m.name === sup;
      return `<div class="gnode ${st.graphSel === m.name ? "is-sel" : ""} ${isRoot ? "is-root" : ""}"
        data-gnode="${m.name}" tabindex="0" role="button" aria-label="${m.name}，${m.role}"
        style="left:${p.x}px;top:${p.y}px">
        <span class="gav">${m.name.slice(0, 1)}</span>
        <span class="gmeta"><b>${m.name}</b><span>${isRoot ? "主管 · " : ""}${m.role}</span></span>
        <span class="gdot ${dot}" title="${stMap[1]}"></span>
        <span class="gnode-actions">
          <button data-graph-action="add-child" data-name="${m.name}" title="添加下属" aria-label="为 ${m.name} 添加下属">${I("plus", "sm")}</button>
          <button data-graph-action="node-menu" data-name="${m.name}" title="更多操作" aria-label="${m.name} 的更多操作">${I("more", "sm")}</button>
        </span>
      </div>`;
    }).join("");

    return `<div class="card" style="display:flex;flex-direction:column;flex:1 1 auto;min-height:0;overflow:hidden">
      <div class="graph-toolbar">
        <span class="tiny muted">拖动空白平移 · 滚轮缩放 · 拖动节点调整位置</span>
        <span class="spacer"></span>
        <button class="icon-btn" data-action="graph-zoom" data-dir="-1" aria-label="缩小" title="缩小">${I("left", "sm")}</button>
        <span class="graph-zoom">${Math.round(st.graphZoom * 100)}%</span>
        <button class="icon-btn" data-action="graph-zoom" data-dir="1" aria-label="放大" title="放大">${I("right", "sm")}</button>
        <button class="btn is-secondary sm" data-action="graph-fit">适应视图</button>
        <button class="btn is-secondary sm" data-action="graph-tidy">${I("refresh", "sm")}自动整理</button>
        <button class="btn is-primary sm" data-action="graph-new-agent">${I("plus", "sm")}添加 Agent</button>
      </div>
      <div class="graph-viewport" id="graph-viewport">
        <div class="graph-plane" id="graph-plane" style="transform:translate(${st.graphPan.x}px, ${st.graphPan.y}px) scale(${st.graphZoom})">
          <svg class="graph-edges" width="${maxX}" height="${maxY}" viewBox="0 0 ${maxX} ${maxY}">${edges}</svg>
          ${nodes}
        </div>
        <div class="graph-hint">${I("info", "sm")}共 ${team.members.length} 个节点 · 主管：${sup} · 实线 = 直属汇报关系 · 悬停节点显示 + / ⋯</div>
      </div>
    </div>`;
  };

  /* ---------- teams ---------- */
  S.teams = function (st) {
    const team = st.teams.find(t => t.id === st.teamId) || st.teams[0];
    const tree = st.teams.map(t => {
      const on = t.id === team.id;
      return `<button class="row" style="gap:8px;padding:7px 9px;border-radius:6px;text-align:left;width:100%;${on ? "background:var(--bg-subtle);font-weight:600" : ""}" data-action="select-team" data-id="${t.id}">
        ${I(on ? "down" : "right", "sm muted")}${I("users", "sm secondary")}
        <span style="flex:1 1 auto">${t.name}</span><span class="tiny muted">${t.members.length}</span></button>
        ${on ? t.members.map(m => `<div class="row" style="gap:7px;padding:5px 8px 5px 26px">
          <span class="ib" style="width:20px;height:20px;border-radius:5px;background:var(--bg-subtle);display:grid;place-items:center;font-size:9.5px;font-weight:600">${m.name.slice(0, 1)}</span>
          <span class="col" style="gap:0px;flex:1 1 auto;min-width:0"><span class="small">${m.name}</span><span class="tiny muted">${m.role}</span></span>
          ${B(AGENT_STATE[m.state][0], AGENT_STATE[m.state][1])}
          <button class="icon-btn" style="width:22px;height:22px" data-action="menu-member" data-name="${m.name}" aria-label="${m.name} 操作" title="成员操作">${I("more", "sm")}</button>
        </div>`).join("") : ""}`;
    }).join("");

    const memberRows = team.members.map(m => `<tr>
      <td>${m.name}</td><td class="small secondary">${m.reports}</td><td class="small secondary">${m.tool}</td>
      <td>${B(AGENT_STATE[m.state][0], AGENT_STATE[m.state][1])}</td>
      <td><button class="icon-btn" data-action="menu-member" data-name="${m.name}" aria-label="${m.name} 操作">${I("more", "sm")}</button></td></tr>`).join("");


    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">Agent 团队</h1>
          <p class="page-sub">谁向谁汇报。上下级关系用树形结构维护；关系图只用于定位与查看，不是唯一入口。</p></div>
        <div class="head-actions">
          ${BT({ label: "新建团队", variant: "primary", action: "dialog-new-team", icon: "plus" })}
        </div>
      </div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="card col col-list" style="padding:8px;gap:2px;overflow:auto">${tree}</div>
        <div class="grow card detail-panel">
          <div class="row" style="gap:9px">
            <span class="section-title" style="font-size:17px">${team.name}</span>
            ${st.teamWarning ? B("blocked", "关系检查未通过") : B("success", "关系检查通过")}
            <span class="spacer"></span>
            <div class="segmented">
              <button class="${st.teamView === "tree" ? "is-on" : ""}" data-action="team-view" data-view="tree">树形</button>
              <button class="${st.teamView === "graph" ? "is-on" : ""}" data-action="team-view" data-view="graph">关系图</button>
            </div>
            ${BT({ label: "编辑关系", variant: "secondary", size: "sm", action: "dialog-team-edit" })}
            ${BT({ label: "添加成员", variant: "primary", size: "sm", action: "menu-add-member" })}
          </div>
          <div class="tiny muted">${team.members.length} 名成员 · 主管：${team.supervisor} · 默认工作空间 ${team.ws} · 团队级覆盖：2 项权限</div>
          ${st.teamWarning
            ? `<div class="banner is-blocked">${I("alert", "sm")}<span style="flex:1 1 auto">${st.teamWarning}</span></div>`
            : `<div class="banner is-blocked">${I("alert", "sm")}<span style="flex:1 1 auto">关系检查：未发现自我管理或间接环。尚未就绪的成员会在任务执行到该角色时停下并提示，不会跳过。</span></div>`}
          ${st.teamView === "graph" ? S.orgChart(st, team) : `<table class="table" style="border:1px solid var(--border-default);border-radius:6px;overflow:hidden">
                <thead><tr><th>成员</th><th>直属主管</th><th>执行工具</th><th>状态</th><th></th></tr></thead>
                <tbody>${memberRows}</tbody></table>`}
          <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}
            <span class="tiny">保存只代表配置已保存；正在运行的 Run 不会静默采用新配置，改动从下一个新任务/新 Run 开始生效。</span></div>
        </div>
      </div>
    `, { page: "Agent 角色" });
  };

  /* ---------- teams ---------- */
  /* ---------- workspaces ---------- */
  S.workspaces = function (st) {
    const list = st.workspaces.map(w => {
      const on = w.id === st.wsId;
      const kind = w.state === "conflict" ? ["blocked", "2 任务共用"] : w.state === "readonly" ? ["neutral", "只读挂载"] : ["success", "可用"];
      return `<button class="card" style="padding:12px;display:flex;flex-direction:column;gap:5px;text-align:left;${on ? "border:2px solid var(--text-primary);padding:11px" : ""}" data-action="select-ws" data-id="${w.id}">
        <span class="row" style="gap:7px">${I("folder", "sm secondary")}<span class="mono" style="font-size:11.5px;flex:1 1 auto">${w.name}</span>${B(kind[0], kind[1])}</span>
        <span class="row" style="gap:8px"><span class="mono tiny muted">${w.path}</span><span class="tiny muted">· ${w.tasks} 个任务</span></span>
      </button>`;
    }).join("");

    const w = st.workspaces.find(x => x.id === st.wsId) || st.workspaces[0];
    const scopes = [["可读取", "eye", w.read.join(" · ") || "—"], ["可写入", "pencil", w.write.join(" · ") || "无"], ["禁止访问", "lock", w.deny.join(" · ")]]
      .map(([t, ic, d]) => `<div class="card panel" style="gap:4px;background:var(--bg-inset)">
        <span class="row" style="gap:6px">${I(ic, "sm")}<b class="small">${t}</b></span>
        <span class="mono tiny muted" style="line-height:1.45">${d}</span></div>`).join("");

    const related = st.tasks.filter(t => t.ws === w.name).map(t => {
      const [k, l] = UI.taskStateLabel(t.state);
      return `<button class="row" style="gap:8px;padding:7px 9px;background:var(--bg-inset);border-radius:5px;width:100%;text-align:left" data-action="open-task" data-id="${t.id}">
        <span class="small" style="flex:1 1 auto">${t.name}</span>${B(k, l)}</button>`;
    }).join("") || `<div class="tiny muted">此工作空间暂无任务。</div>`;

    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">工作空间</h1>
          <p class="page-sub">任务能读写的本地目录范围。边界在这里定义，Agent 只能看到被授权的部分。</p></div>
        <div class="head-actions">${BT({ label: "添加工作空间", variant: "primary", action: "dialog-new-workspace", icon: "plus" })}</div>
      </div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="col col-list" style="gap:8px;overflow:auto">${list}</div>
        <div class="grow card detail-panel">
          <div class="row" style="gap:8px;flex-wrap:wrap">
            <span class="mono" style="font-size:16px;font-weight:600">${w.name}</span>
            <span class="mono tiny muted">${w.path}</span><span class="spacer"></span>
            ${BT({ label: "打开路径", variant: "secondary", size: "sm", action: "dialog-open-folder", icon: "external" })}
            ${BT({ label: "编辑边界", variant: "primary", size: "sm", action: "dialog-ws-boundary" })}
          </div>
          ${w.state === "conflict" ? `<div class="banner is-blocked" style="align-items:flex-start">${I("alert", "sm")}
            <span class="col" style="gap:3px;flex:1 1 auto">
              <b class="small">检测到文件冲突风险：2 个任务正在写入同一目录</b>
              <span class="tiny" style="line-height:1.5">Run #1244 与 Run #1240 都会写入 terms/。系统不会假定已实现隔离，请选择处理方式。</span>
            </span>
            ${BT({ label: "排队其中一个", variant: "secondary", size: "sm", action: "resolve-conflict" })}
          </div>` : ""}
          <div class="grid-3">${scopes}</div>
          <div class="row" style="gap:12px;align-items:flex-start">
            <div class="col" style="flex:1 1 auto;gap:6px;min-width:0"><span class="section-title">相关任务</span>${related}</div>
            <div class="col" style="width:260px;flex:0 0 260px;gap:6px"><span class="section-title">使用此空间的团队</span>
              ${["数据小组（2 名成员）", "法务助手（单 Agent）"].map(x => `<div class="row" style="gap:7px;padding:7px 9px;background:var(--bg-inset);border-radius:5px">${I("users", "sm muted")}<span class="small secondary">${x}</span></div>`).join("")}
            </div>
          </div>
        </div>
      </div>
    `, { page: "工作空间" });
  };

  /* ---------- approvals ---------- */
  S.approvals = function (st) {
    const pending = st.approvals.filter(a => a.state === "pending");
    const list = st.approvals.filter(a => st.approvalFilter === "待处理" ? a.state === "pending" : true).map(a => {
      const on = a.id === st.approvalId;
      const kind = a.state === "pending" ? ["blocked", "等待审批"] : a.state === "expired" ? ["neutral", "已过期"] : a.state === "denied" ? ["failure", "已拒绝"] : ["success", "已允许"];
      return `<button class="card" style="padding:12px;display:flex;flex-direction:column;gap:6px;text-align:left;${on ? "border:2px solid var(--text-primary);padding:11px" : ""}" data-action="select-approval" data-id="${a.id}">
        <span class="row" style="gap:7px">${B(kind[0], kind[1])}<span class="spacer"></span><span class="tiny muted">${a.age}</span></span>
        <span class="mono" style="font-size:11px">${a.title}</span>
        <span class="tiny muted">${a.agent}</span>
      </button>`;
    }).join("");

    const a = st.approvals.find(x => x.id === st.approvalId) || pending[0] || st.approvals[0];
    const task = st.tasks.find(t => t.id === a.taskId);
    const isPending = a.state === "pending";
    const badge = isPending ? B("blocked", "等待审批") : a.state === "expired" ? B("neutral", "已过期") : a.state === "denied" ? B("failure", "已拒绝") : B("success", "已允许");

    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">待审批</h1>
          <p class="page-sub">集中处理 Agent 等待你决定的请求。任务内的审批卡与本页指向同一条记录，不会产生两份。</p></div>
        <div class="head-actions"><span class="tiny muted">默认不允许，需要你明确选择</span></div>
      </div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="col col-list" style="gap:8px;overflow:auto">
          <div class="row" style="gap:6px">
            ${["待处理", "已处理"].map(f => `<button class="chip ${st.approvalFilter === f ? "is-on" : ""}" data-action="approval-filter" data-filter="${f}" style="height:24px;font-size:11px">${f}${f === "待处理" ? ` ${pending.length}` : ""}</button>`).join("")}
          </div>
          ${list}
        </div>
        <div class="grow card detail-panel">
          <div class="row" style="gap:8px;align-items:flex-start">
            <span class="col" style="gap:2px;flex:1 1 auto;min-width:0">
              <b style="font-size:16px">${a.title}</b>
              <span class="tiny muted">属于任务：${task ? task.name : "—"} · Run ${task ? task.run : ""}</span>
            </span>
            ${badge}
            ${BT({ label: "查看任务", variant: "ghost", size: "sm", action: "open-task", data: { id: a.taskId }, icon: "upright" })}
          </div>
          <div class="code-block">
            <div class="tiny muted" style="margin-bottom:4px">请求的操作</div>
            <div>${esc(a.target)}</div>
            <div class="muted" style="margin-top:4px">${esc(a.diff)}</div>
          </div>
          <div class="col" style="gap:5px">
            <span class="section-title" style="font-size:11.5px">为什么需要你确认</span>
            ${a.risk.map(r => `<div class="row" style="gap:7px;align-items:flex-start">${I("dot", "sm")}<span class="small secondary" style="line-height:1.5">${r}</span></div>`).join("")}
          </div>
          <div class="banner is-neutral">${I("shield", "sm")}<span class="small" style="flex:1 1 auto">授权作用域：${a.scope}</span></div>
          ${isPending ? `<div class="row" style="gap:8px;flex-wrap:wrap">
              ${BT({ label: "拒绝", variant: "secondary", action: "deny-approval", data: { id: a.id }, icon: "x" })}
              ${BT({ label: "仅此一次允许", variant: "primary", action: "allow-approval", data: { id: a.id }, icon: "check" })}
              <span class="chip" style="color:var(--text-muted);cursor:not-allowed" title="需要可靠的授权范围与撤销能力，首版不提供">本次任务内允许（需可靠撤销）· 规划中</span>
            </div>` : `<div class="banner is-neutral">${I("info", "sm")}<span class="small">该请求已有最终记录，不再提供批准入口。需要重新执行时请从任务中发起新的请求。</span></div>`}
          <div class="col" style="gap:7px">
            <span class="section-title" style="font-size:11.5px">记录</span>
            ${a.history.map(([t, title, d]) => `<div class="row" style="gap:9px;align-items:flex-start">
              <span class="mono tiny muted" style="width:52px">${t}</span>
              <span class="col" style="gap:1px"><b class="small">${title}</b><span class="tiny muted">${d}</span></span></div>`).join("")}
          </div>
        </div>
      </div>
    `, { page: "待审批" });
  };

  /* ---------- history ---------- */
  S.history = function (st) {
    const r = st.filters;
    const extra = Array.from({ length: st.historyMore }, (_, i) => ({
      id: "#" + (1240 - i), task: "整理历史资料（更早的 Run）", who: "数据小组", state: ["success", "stopped", "unknown"][i % 3], start: "昨天", dur: "2m"
    }));
    const allRuns = DATA.runs.concat(extra).filter(x => {
      if (r.runType === "仅失败" && x.state !== "failure") return false;
      if (r.runResult !== "全部" && RUN_STATE[x.state][1] !== r.runResult) return false;
      return true;
    });
    const rows = allRuns.map(x => {
      const [k, l] = RUN_STATE[x.state];
      return `<tr class="clickable ${x.id === st.runId ? "is-selected" : ""}" data-action="select-run" data-id="${x.id}" tabindex="0">
        <td class="mono small">${x.id}</td><td>${x.task}<div class="tiny muted">${x.who}</div></td>
        <td>${B(k, l)}</td><td class="mono tiny muted">${x.start}</td><td class="mono tiny muted">${x.dur}</td>
        <td><button class="icon-btn" data-action="menu-run" data-id="${x.id}" aria-label="Run 操作" title="Run 操作">${I("more", "sm")}</button></td></tr>`;
    }).join("");

    const p = DATA.partialRun;
    const groups = p.groups.map(g => `<div class="col" style="gap:5px">
      <div class="row" style="gap:6px">${B(g.kind, g.label)}<span class="tiny muted">${g.items.length} 项</span></div>
      ${g.items.map(([f, d]) => `<div class="row" style="gap:7px;padding:6px 8px;background:var(--bg-inset);border-radius:4px">
        ${I("file", "sm muted")}<span class="mono tiny" style="flex:1 1 auto;min-width:0">${f}</span><span class="tiny muted">${d}</span></div>`).join("")}
    </div>`).join("");

    return S.shell(st, `
      <div class="page-head">
        <div><h1 class="page-title">历史记录</h1>
          <p class="page-sub">追溯每一次 Run 与关键操作。重试会创建新的 Run，旧日志与产物保持不变。</p></div>
        <div class="head-actions">${BT({ label: "导出记录", variant: "secondary", size: "sm", action: "export-history", icon: "file" })}</div>
      </div>
      <div class="row" style="gap:8px;flex-wrap:wrap">
        <label class="searchbox" style="min-width:240px">${I("search", "sm")}
          <input id="run-search" placeholder="搜索 Run 编号或任务" aria-label="搜索 Run"
            style="border:0;background:transparent;outline:none;font-size:12px;width:100%" /></label>
        ${BT({ label: "类型：" + r.runType, variant: "secondary", size: "sm", action: "menu-filter", data: { kind: "run-type" } })}
        ${BT({ label: "结果：" + r.runResult, variant: "secondary", size: "sm", action: "menu-filter", data: { kind: "run-result" } })}
        ${BT({ label: "时间：" + r.runTime, variant: "secondary", size: "sm", action: "menu-filter", data: { kind: "run-time" } })}
        <span class="spacer"></span>
        <button class="btn is-ghost sm" data-action="clear-run-filter">清除筛选</button>
      </div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="grow table-wrap" style="overflow:auto;min-width:0">
          <table class="table"><thead><tr><th>Run</th><th>任务与执行者</th><th>结果</th><th>开始</th><th>耗时</th><th></th></tr></thead>
          <tbody>${rows || `<tr><td colspan="6"><div class="empty"><span class="et">没有匹配的 Run</span><span class="small">试试清除筛选条件。</span></div></td></tr>`}</tbody></table>
          <div class="row" style="justify-content:center;padding:10px">
            <button class="btn is-ghost sm" data-action="history-more">加载更多${st.historyMore ? "（已加载 " + st.historyMore + " 条）" : "（还有 24 条）"}</button>
          </div>
        </div>
        <div class="card col-rail detail-panel">
          <div class="row" style="gap:7px"><b class="mono" style="font-size:13px">${st.runId}</b>${B("blocked", "部分完成")}</div>
          <div class="small secondary">批量规范化 320 条术语词表</div>
          <div class="banner is-blocked" style="align-items:flex-start">${I("alert", "sm")}
            <span class="tiny" style="line-height:1.5">部分成果：5 个子任务中 3 项完成、1 项跳过、1 项失败。整体不算成功。</span></div>
          ${groups}
          <div class="row" style="gap:7px;flex-wrap:wrap">
            ${BT({ label: "导出部分成果", variant: "secondary", size: "sm", action: "export-partial" })}
            ${BT({ label: "在新 Run 中重试", variant: "primary", size: "sm", action: "retry-new-run" })}
          </div>
          <div class="tiny muted">重试将创建新的 Run，并保留 ${st.runId} 的全部日志与产物。</div>
        </div>
      </div>
    `, { page: "历史记录" });
  };

  /* ---------- settings ---------- */
  S.settings = function (st) {
    const sections = [["外观", "sun"], ["执行与资源", "sliders"], ["通知", "bell"], ["权限与安全", "shield"], ["关于", "info"]];
    const nav = sections.map(([name, ic]) => `<button class="nav-item ${st.settingsSection === name ? "is-active" : ""}" data-action="settings-section" data-section="${name}">${I(ic)}<span class="label">${name}</span></button>`).join("");

    const setRow = (title, desc, control) => `<div class="row" style="gap:12px;align-items:center">
      <span class="col" style="gap:2px;flex:1 1 auto;min-width:0"><b class="small">${title}</b><span class="tiny muted" style="line-height:1.45">${desc}</span></span>
      ${control}
    </div>`;

    let panel = "";
    if (st.settingsSection === "外观") {
      panel = `<span class="section-title" style="font-size:15px">外观</span>
        ${setRow("主题", "跟随系统时，亮暗主题随 macOS / Windows 设置切换，主色始终保持黑白灰。", `<div class="segmented">
          ${["跟随系统", "亮色", "暗色"].map(x => `<button class="${st.themePref === x ? "is-on" : ""}" data-action="theme-pref" data-pref="${x}">${x}</button>`).join("")}</div>`)}
        ${setRow("界面密度", "舒适使用 13px 正文与 34px 行高；紧凑使用 12.5px 正文与 30px 行高。", `<div class="segmented">
          ${["舒适", "紧凑"].map(x => `<button class="${st.density === (x === "紧凑" ? "compact" : "comfortable") ? "is-on" : ""}" data-action="density-pref" data-value="${x === "紧凑" ? "compact" : "comfortable"}">${x}</button>`).join("")}</div>`)}
        ${setRow("减少动态效果", "关闭过渡动画与滚动惯性，仅保留必要反馈。", `<button class="switch ${st.motion === "reduced" ? "is-on" : ""}" data-action="toggle-motion" role="switch" aria-checked="${st.motion === "reduced"}"></button>`)}
        ${setRow("灰度预览（可访问性走查）", "用灰度模拟色觉差异，验证状态不只依赖颜色。", `<button class="switch ${st.grayscale ? "is-on" : ""}" data-action="toggle-grayscale" role="switch" aria-checked="${st.grayscale}"></button>`)}
        ${setRow("窗口外框预览", "检查 macOS 与 Windows 标题栏控件安全区、拖动区与工具栏是否冲突。", `<div class="segmented">
          <button class="${st.platform === "mac" ? "is-on" : ""}" data-action="platform" data-value="mac">macOS</button>
          <button class="${st.platform === "windows" ? "is-on" : ""}" data-action="platform" data-value="windows">Windows</button></div>`)}
        <div class="divider"></div>
        <span class="section-title" style="font-size:15px">窗口与退出</span>
        ${[["仅关闭窗口", "任务在后台继续运行；再次打开应用可回到同一任务。", st.closeBehavior === "window"],
           ["退出应用", "退出前提示如何处理运行中的任务：停止、等待或在下次启动时恢复。", st.closeBehavior === "app"]]
          .map(([t, d, on]) => `<button class="row" style="gap:9px;align-items:flex-start;text-align:left" data-action="close-behavior" data-value="${t === "退出应用" ? "app" : "window"}">
            <span style="width:15px;height:15px;border-radius:50%;border:1.5px solid ${on ? "var(--text-primary)" : "var(--border-strong)"};display:grid;place-items:center;flex:0 0 auto;margin-top:2px">
              ${on ? `<span style="width:7px;height:7px;border-radius:50%;background:var(--text-primary)"></span>` : ""}</span>
            <span class="col" style="gap:1px"><b class="small">${t}</b><span class="tiny muted" style="line-height:1.45">${d}</span></span></button>`).join("")}
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}
          <span class="tiny">原型说明：后台运行能力取决于核心进程的部署方式（演示中仅模拟提示），不对此作出承诺。</span></div>`;
    } else if (st.settingsSection === "执行与资源") {
      panel = `<span class="section-title" style="font-size:15px">执行与资源</span>
        ${setRow("并发上限", "达到上限时新任务排队并说明原因，不静默丢弃。", `<div class="row" style="gap:6px">
          ${BT({ label: "−", variant: "secondary", size: "sm", action: "concurrency", data: { delta: -1 } })}
          <span class="mono" style="width:26px;text-align:center">${st.concurrency}</span>
          ${BT({ label: "+", variant: "secondary", size: "sm", action: "concurrency", data: { delta: 1 } })}</div>`)}
        ${setRow("自动重试", "仅当能证明尚未进入真实执行且无副作用时才允许；默认关闭。", `<button class="switch ${st.autoRetry ? "is-on" : ""}" data-action="toggle-autoretry" role="switch" aria-checked="${st.autoRetry}"></button>`)}
        ${setRow("日志保留", "Run 日志与产物保留在本机。", BT({ label: st.logRetention, variant: "secondary", size: "sm", action: "menu-retention", icon: "down" }))}
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}
          <span class="tiny">演示中调整并发只会影响提示文案，不会真正启动或停止任何进程。</span></div>`;
    } else if (st.settingsSection === "通知") {
      panel = `<span class="section-title" style="font-size:15px">通知</span>
        ${[["审批请求", "有 Agent 等待你决定时提醒。", "approval"], ["任务失败或阻塞", "执行失败、依赖阻塞、状态待确认。", "failure"], ["任务完成", "任务完成后提醒一次。", "done"], ["仅显示系统通知", "关闭应用内未读标记。", "systemOnly"]]
          .map(([t, d, key]) => setRow(t, d, `<button class="switch ${st.notifPrefs[key] ? "is-on" : ""}" data-action="toggle-notif-pref" data-key="${key}" role="switch" aria-checked="${st.notifPrefs[key]}"></button>`)).join("")}`;
    } else if (st.settingsSection === "权限与安全") {
      panel = `<span class="section-title" style="font-size:15px">权限与安全</span>
        ${setRow("默认审批策略", "高风险写入、删除与命令执行始终需要明确授权。", `<span class="chip">默认不允许</span>`)}
        ${setRow("授权作用域", "审批记录包含作用域与结果，可在“待审批”查看历史。", BT({ label: "查看审批记录", variant: "secondary", size: "sm", action: "go", data: { screen: "approvals" } }))}
        ${setRow("凭据", "VocaMesh 不保存 API Key；登录由各 CLI 自行管理。", `<span class="chip">仅本地</span>`)}
        <div class="banner is-neutral" style="align-items:flex-start">${I("lock", "sm")}
          <span class="tiny">企业版预留：管理员策略、审批角色与审计导出。当前原型不包含多租户管理。</span></div>`;
    } else {
      panel = `<span class="section-title" style="font-size:15px">关于</span>
        <div class="kv"><span class="k">产品</span><span>VocaMesh · by Vocafect</span></div>
        <div class="kv"><span class="k">版本</span><span class="mono">UX/UI prototype v1.2（设计交付）</span></div>
        <div class="kv"><span class="k">数据</span><span>全部为虚构演示数据，不调用真实 CLI 或文件系统</span></div>
        <div class="kv"><span class="k">导出记录</span><span>${st.downloads.length ? st.downloads.map(d => d.filename).join("、") : "本次会话尚未导出文件"}</span></div>
        <div class="kv"><span class="k">反馈</span><span>${st.feedback.length ? "已记录 " + st.feedback.length + " 条（本机）" : "尚未提交"}</span></div>
        <div class="row" style="gap:8px;flex-wrap:wrap">
          ${BT({ label: "重置演示数据", variant: "secondary", size: "sm", action: "reset-demo", icon: "refresh" })}
          ${BT({ label: "键盘快捷键", variant: "ghost", size: "sm", action: "show-shortcuts" })}
          ${BT({ label: "查看帮助", variant: "ghost", size: "sm", action: "go", data: { screen: "help" } })}</div>`;
    }

    return S.shell(st, `
      <div class="page-head"><div><h1 class="page-title">设置</h1>
        <p class="page-sub">更改会立即生效；高风险选项会说明生效范围与恢复方式。</p></div></div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="col col-nav" style="gap:2px">${nav}</div>
        <div class="grow card detail-panel">
          ${panel}
          <span class="spacer"></span>
          <div class="row" style="gap:8px"><span class="tiny muted">更改已自动保存</span><span class="spacer"></span>
            ${BT({ label: "恢复默认外观", variant: "ghost", size: "sm", action: "reset-appearance" })}</div>
        </div>
      </div>
    `, { page: "设置" });
  };

  /* ---------- state gallery ---------- */
  S.states = function (st) {
    const cards = [
      ["E01", "未发现任何可运行工具", "工具显示“未安装 / 无法检测”，任务按钮禁用并说明原因。", "neutral", "未检测到", [["去本地工具", "go", { screen: "tools" }], ["手动选择路径", "dialog-tool-path", { id: "opencode" }]]],
      ["E05", "下属执行失败 · 依赖阻塞", "失败节点红色，依赖节点琥珀“已阻塞”，无关任务继续。", "failure", "执行失败", [["打开失败任务", "open-task", { id: "t5" }]]],
      ["E08", "用户点击暂停", "“等待暂停：已停止派发，当前步骤仍在运行”。", "neutral", "等待暂停", [["在任务中暂停", "pause-task", {}]]],
      ["E10", "停止请求未确认", "“正在请求停止”与“已停止”分开；超时进入“状态待确认”。", "unknown", "状态待确认", [["打开检查器", "inspector-open", { tab: "activity" }], ["查看历史", "go", { screen: "history" }]]],
      ["E12", "并发资源已满", "任务排队并给出可解释原因，不静默丢弃。", "neutral", "排队等待资源", [["调整并发上限", "go", { screen: "settings" }]]],
      ["E13", "多任务写入同一目录", "冲突提醒 + 工作空间边界 + 隔离选项。", "blocked", "文件冲突风险", [["查看工作空间", "go", { screen: "workspaces" }]]],
      ["E15", "部分成果交付", "按完成 / 跳过 / 失败分组，标注缺失范围。", "blocked", "部分完成", [["查看历史记录", "go", { screen: "history" }]]],
      ["E17", "无任务 · 无团队 · 无记录", "空状态提供明确下一步，而不是空白页。", "neutral", "暂无内容", [["新建任务", "focus-composer", {}], ["查看入门指南", "go", { screen: "help" }]]],
      ["E18", "窄窗 / 长文本 / 大量日志", "文本换行、区域独立滚动、日志可搜索复制。", "neutral", "已截断显示", [["打开日志", "inspector-open", { tab: "logs" }], ["调整密度", "go", { screen: "settings" }]]]
    ];
    return S.shell(st, `
      <div class="page-head"><div><h1 class="page-title">异常与空状态合集</h1>
        <p class="page-sub">同一个“失败弹窗”不能满足全部场景：每种状态都说明发生了什么、影响范围与下一步。</p></div>
        <div class="head-actions"><span class="mono tiny muted">E01 · E05 · E08 · E10 · E12 · E13 · E15 · E17 · E18</span></div></div>
      <div class="grid-3" style="flex:1 1 auto;min-height:0;overflow:auto;align-content:start">
        ${cards.map(([code, title, desc, kind, label, actions]) => `
          <div class="card panel" style="gap:7px;justify-content:space-between">
            <div class="row" style="gap:7px"><span class="mono tiny muted">${code}</span><span class="spacer"></span>${B(kind, label)}</div>
            <b style="font-size:12.5px">${title}</b>
            <span class="tiny secondary" style="line-height:1.5">${desc}</span>
            <div class="row" style="gap:6px;flex-wrap:wrap">${actions.map(([l, action, data], i) => BT({ label: l, variant: i === 0 ? "secondary" : "ghost", size: "sm", action, data })).join("")}</div>
          </div>`).join("")}
      </div>
    `, { page: "异常与空状态" });
  };

  /* ---------- notifications ---------- */
  S.notifications = function (st) {
    const unread = st.notifications.filter(n => n.unread).length;
    const items = st.notifications.filter(n => st.notifFilter === "未读" ? n.unread : st.notifFilter === "审批" ? n.kind === "blocked" : true);
    return S.shell(st, `
      <div class="page-head"><div><h1 class="page-title">通知</h1>
        <p class="page-sub">需要你关注的任务事件。通知只负责告诉你，审批操作仍在“待审批”中完成。</p></div>
        <div class="head-actions">${BT({ label: "全部标为已读", variant: "secondary", action: "mark-all-read", icon: "checkcheck", disabled: !unread })}</div></div>
      <div class="tabs" style="padding:0">
        ${["全部", "未读", "审批"].map(f => `<button class="${st.notifFilter === f ? "is-on" : ""}" data-action="notif-filter" data-filter="${f}">${f}${f === "未读" && unread ? ` · ${unread}` : ""}</button>`).join("")}
      </div>
      <div class="card" style="flex:1 1 auto;overflow:auto">
        ${items.map(n => `<div class="row" style="gap:11px;padding:11px 14px;border-bottom:1px solid var(--border-default);background:${n.unread ? "var(--bg-subtle)" : "transparent"}">
          <span class="ib" style="width:26px;height:26px;border-radius:6px;background:var(--bg-surface);border:1px solid var(--border-default);display:grid;place-items:center;color:${n.kind === "failure" ? "var(--status-failure)" : n.kind === "success" ? "var(--status-success)" : n.kind === "blocked" ? "var(--status-blocked)" : "var(--text-secondary)"}">
            ${I(n.kind === "failure" ? "x" : n.kind === "success" ? "check" : n.kind === "blocked" ? "shield" : "pause", "sm")}</span>
          <button class="col" style="gap:2px;flex:1 1 auto;min-width:0;text-align:left" data-action="open-notification" data-id="${n.id}">
            <b class="small" style="font-weight:${n.unread ? 600 : 500}">${n.title}</b>
            <span class="tiny muted">${n.body}</span>
          </button>
          <span class="tiny muted">${n.time}</span>
          ${BT({ label: "查看", variant: "ghost", size: "sm", action: "open-notification", data: { id: n.id } })}
          <button class="icon-btn" data-action="dismiss-notification" data-id="${n.id}" aria-label="移除通知" title="移除">${I("x", "sm")}</button>
          ${n.unread ? `<span style="width:6px;height:6px;border-radius:50%;background:var(--text-primary)"></span>` : ""}
        </div>`).join("") || `<div class="empty"><span class="et">没有未读通知</span><span class="small">新的审批请求与任务事件会出现在这里。</span></div>`}
      </div>
    `, { page: "通知" });
  };

  /* ---------- help ---------- */
  S.help = function (st) {
    const articles = [
      ["入门", ["开始你的第一项任务", "Agent 术语速查"]],
      ["配置", ["安装与认证本地 CLI", "工具与角色的区别"]],
      ["任务", ["暂停、停止与重试的区别", "审批如何工作", "部分成果与失败"]],
      ["支持", ["反馈与问题", "键盘快捷键"]]
    ];
    const q = (st.helpQuery || "").toLowerCase();
    const nav = articles.map(([g, items]) => {
      const filtered = items.filter(t => !q || t.toLowerCase().includes(q));
      if (!filtered.length) return "";
      return `<div class="tiny muted" style="padding:8px 8px 4px;font-weight:600;letter-spacing:.5px">${g}</div>
        ${filtered.map(t => `<button class="row" style="gap:6px;padding:6px 8px;border-radius:5px;width:100%;text-align:left;font-size:11.5px;${t === st.helpArticle ? "background:var(--bg-subtle);font-weight:600" : "color:var(--text-secondary)"}" data-action="help-article" data-article="${t}">${t}</button>`).join("")}`;
    }).join("");

    const bodyMap = {
      "开始你的第一项任务": `<h2>开始你的第一项任务</h2>
        <p>VocaMesh 的入口只有一个问题：“你想完成什么”。先描述任务，再决定用哪个 Agent 或团队执行。</p>
        <ol>
          <li><b>检查可用的执行工具</b>：左侧 → 本地工具。看到“已安装且可用”才能被执行；显示“认证待验证”时，先在终端完成该 CLI 的登录。VocaMesh 不保存你的凭据。</li>
          <li><b>创建 Agent 角色</b>：左侧 → Agent 角色 → 新建 Agent。只需名称、职责、执行工具三项即可保存。一个工具可以服务多个角色。</li>
          <li><b>描述任务并创建</b>：回到工作台，输入任务、确认工作空间与执行方式，然后创建任务。创建后会先排队。</li>
          <li><b>处理审批与异常</b>：需要授权时任务会停下，任务详情与左侧“待审批”都能处理。失败时可以只重试失败部分，或跳过并生成标注完整性的部分成果。</li>
        </ol>
        <div class="banner is-neutral" style="align-items:flex-start">${I("bulb", "sm")}<span class="tiny">容易混淆的三组概念：本地工具 = 可运行的环境；Agent 角色 = 谁来做；Agent 团队 = 谁向谁汇报。任务依赖与汇报关系无关。</span></div>`,
      "Agent 术语速查": `<h2>Agent 术语速查</h2>
        <ol>
          <li><b>本地工具</b>：可运行的 CLI / 适配器，例如 Claude Code、Codex CLI。</li>
          <li><b>Agent 角色</b>：名称 + 职责 + 执行工具 + 权限。</li>
          <li><b>Agent 团队</b>：角色之间的主管与下属关系。</li>
          <li><b>任务</b>：要达成的目标，可含多个子任务与依赖。</li>
          <li><b>Run</b>：一次执行尝试；重试会产生新的 Run，旧记录不覆盖。</li>
          <li><b>审批</b>：高风险操作的显式授权，默认不允许。</li>
        </ol>`,
      "安装与认证本地 CLI": `<h2>安装与认证本地 CLI</h2>
        <p>VocaMesh 不代替 CLI 完成安装与登录，也不会读取或保存凭据。</p>
        <ol>
          <li>从官方渠道安装 CLI；安装后在“本地工具”点击“重新扫描”。</li>
          <li>在终端中完成该 CLI 自己的登录流程。</li>
          <li>回到“本地工具”点击“测试认证”。验证通过后状态变为“已安装且可用”。</li>
          <li>若状态显示“无法检测”，可手动指定可执行文件路径；仍无法确认时不会假定可用。</li>
        </ol>`,
      "工具与角色的区别": `<h2>工具与角色</h2>
        <p><b>本地工具</b>回答“这台电脑上有什么可运行的环境”；<b>Agent 角色</b>回答“这个环境扮演什么职责”。</p>
        <p>一个工具可以支持多个角色，不需要为每个角色重复安装 CLI。创建角色与登录 CLI 是两个独立步骤。</p>`,
      "暂停、停止与重试的区别": `<h2>暂停、停止与重试</h2>
        <p>这三个动作的语义完全不同，界面会分别说明“请求已发出”和“系统已确认”。</p>
        <ol>
          <li><b>暂停</b>：先停止派发新工作；当前执行到安全边界后才显示“已暂停”。</li>
          <li><b>停止</b>：先显示“正在请求停止”，再由运行时确认“已停止 / 停止失败 / 状态待确认”。</li>
          <li><b>重试</b>：创建新的 Run，保留旧 Run 的日志与产物；不会把未完成的执行“原地继续”。</li>
        </ol>`,
      "审批如何工作": `<h2>审批如何工作</h2>
        <p>高风险操作（覆盖写入、删除、命令执行、访问网络）必须由你明确允许。默认不允许，超时不会变成批准。</p>
        <ol>
          <li>Agent 提出请求，任务进入“等待审批”，停止派发新工作。</li>
          <li>你看到请求内容、授权作用域与风险原因，可拒绝或仅此一次允许。</li>
          <li>决定会写入记录；拒绝后 Agent 不会绕过决定重试。</li>
        </ol>`,
      "部分成果与失败": `<h2>部分成果与失败</h2>
        <p>失败不会掩盖：无关子任务继续，依赖失败节点的子任务阻塞，主管进入“需要处理”。</p>
        <ol>
          <li><b>仅重试失败部分</b>：创建新 Run，只重跑失败与跳过的节点。</li>
          <li><b>跳过并生成部分成果</b>：结果按“已完成 / 跳过 / 失败”分组，并标注缺失范围。</li>
          <li><b>停止整个任务</b>：已完成的成果保留，其余标记为未完成。</li>
        </ol>`,
      "反馈与问题": `<h2>反馈与问题</h2>
        <p>请在下方输入遇到的问题。原型只在本机记录，不会发送到服务器。</p>`,
      "键盘快捷键": `<h2>键盘快捷键</h2>
        <ol>
          <li><b>⌘K / Ctrl+K</b>：命令面板（↑↓ 选择，Enter 执行）。</li>
          <li><b>Esc</b>：关闭菜单 / 命令面板 / 对话框 / 检查器。</li>
          <li><b>Enter</b>：在任务输入框中发送消息（Shift+Enter 换行）。</li>
          <li><b>Tab</b>：在侧栏、按钮和输入框之间移动；焦点始终可见。</li>
        </ol>`
    };

    return S.shell(st, `
      <div class="page-head"><div><h1 class="page-title">帮助与反馈</h1>
        <p class="page-sub">不需要先理解 Agent 术语。这里有最短路径说明、概念对照和常见问题。</p></div>
        <div class="head-actions"><label class="searchbox">${I("search", "sm")}
          <input id="help-search" value="${esc(st.helpQuery)}" placeholder="搜索帮助内容" aria-label="搜索帮助" style="border:0;background:transparent;outline:none;font-size:12px;width:130px" /></label></div></div>
      <div class="stack" style="flex:1 1 auto;min-height:0">
        <div class="card col col-nav" style="width:200px;flex-basis:200px;padding:8px;gap:2px;overflow:auto">${nav || `<div class="tiny muted" style="padding:10px">没有匹配的帮助条目</div>`}</div>
        <div class="grow card article detail-panel">
          ${bodyMap[st.helpArticle] || bodyMap["开始你的第一项任务"]}
          <div class="divider"></div>
          <div class="col" style="gap:7px">
            <div class="row" style="gap:8px"><b class="small">反馈与问题</b><span class="tiny muted">本机演示：不会发送到服务器</span></div>
            <textarea class="textarea" id="help-feedback" style="min-height:52px" placeholder="哪里让你觉得不顺手？例如：不确定“暂停”之后任务还在不在跑。" aria-label="反馈内容"></textarea>
            <div class="row" style="gap:8px"><span class="tiny muted">${st.feedback.length ? "已记录 " + st.feedback.length + " 条：" + esc(st.feedback[st.feedback.length - 1].text.slice(0, 24)) : "可选：附加当前页面信息，不包含日志内容"}</span><span class="spacer"></span>
              ${BT({ label: "提交反馈", variant: "primary", size: "sm", action: "submit-feedback" })}</div>
          </div>
        </div>
      </div>
    `, { page: "帮助与反馈" });
  };

  /* ---------- overlays ---------- */
  const DIALOG_TITLES = {
    "create-task": ["创建任务", "提交后先进入排队，运行环境检查通过才会开始执行。"],
    "new-role": ["新建 Agent 角色", "最少三项即可保存；高级配置可选。"],
    "new-team": ["新建团队", "先设置主管，再添加下属；系统会检查自我管理与间接环。"],
    templates: ["模板库", "选择一个模板填入输入框，可继续修改。"],
    "tool-path": ["手动指定可执行文件路径", "仅在自动检测失败时使用；保存后状态为“无法检测”，需再次验证。"],
    "tool-config": ["工具配置（只读）", "展示适配器读取到的配置摘要，不包含任何凭据内容。"],
    "install-guide": ["安装与认证指南", "在终端中完成，不需要在 VocaMesh 中输入任何密钥。"],
    "tool-error": ["检测错误详情", "可复制给支持人员；不包含凭据。"],
    "confirm-delete-role": ["删除 Agent 角色", "删除后该角色不能再被分配；历史 Run 记录保留。"],
    "workspace-new": ["添加工作空间", "写入范围默认关闭，需要显式开启。"],
    "ws-boundary": ["编辑访问边界", "用逗号分隔多个路径；禁止清单始终生效。"],
    "team-edit": ["编辑团队", "团队级覆盖只影响新任务，不会改变正在运行的 Run。"],
    "file-preview": ["文件预览（模拟）", "原型不读取真实文件，以下为示例内容。"],
    "file-path": ["本地路径", "真实应用中会打开系统文件管理器；原型仅显示路径。"],
    "graph-new": ["在关系图中新建 Agent", "会同时创建 Agent 角色并加入当前团队；直属主管由连线关系决定。"],
    "graph-edit": ["编辑 Agent 信息", "改名会同步更新下属的汇报关系与引用位置。"],
    "graph-superior": ["设置直属上级", "若产生自我管理或间接环，保存会被阻止并说明原因。"],
    shortcuts: ["键盘快捷键", "焦点始终可见；Esc 关闭当前弹层。"]
  };

  function dialogBody(st) {
    const d = st.dialog;
    const data = st.dialogData || {};
    if (d === "templates") {
      return `<div class="col" style="gap:8px">
        ${[["竞品对比报告", "把 2025 Q3 三家竞品的公开资料整理成对比报告，标注来源，缺数据留空。", "选择团队"],
           ["术语表规范化", "规范化 terms/ 下的术语词表，保持术语一致性并输出变更清单。", "快速执行"],
           ["更新日志双版本文案", "根据本次变更清单生成中英文 changelog 草稿，并校对术语。", "选择团队"]]
          .map(([t, txt, mode]) => `<button class="card panel" style="text-align:left;gap:5px" data-action="use-template" data-text="${esc(txt)}" data-mode="${mode}">
            <b class="small">${t}</b><span class="tiny muted" style="line-height:1.5">${txt}</span>
            <span class="tiny muted">执行方式：${mode}</span></button>`).join("")}
      </div>`;
    }
    if (d === "tool-path") {
      const tool = st.tools.find(t => t.id === data.id) || st.tools[0];
      return `<div class="field"><span class="lbl">可执行文件路径</span>
        <input class="input" id="tp-path" placeholder="/usr/local/bin/claude" value="${esc(tool.path && tool.path !== "未检测到" ? tool.path : "")}" />
        <span class="hint">Windows 示例：C:\\Users\\me\\AppData\\Local\\Programs\\codex\\codex.exe</span></div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("lock", "sm")}<span class="tiny">只读取路径与版本，不读取该程序配置中的凭据。</span></div>`;
    }
    if (d === "tool-config") {
      const tool = st.tools.find(t => t.id === data.id) || st.tools[0];
      return `<div class="code-block" style="white-space:pre-wrap">${esc(JSON.stringify({ name: tool.name, adapter: tool.adapter, version: tool.version, path: tool.path, state: tool.state, credentials: "not-read", lastCheck: "3 秒前" }, null, 2))}</div>
        <div class="row" style="gap:8px">${BT({ label: "复制配置", variant: "secondary", size: "sm", action: "copy-text", data: { text: tool.name + " · " + tool.adapter + " · " + tool.version, label: "工具信息" } })}</div>`;
    }
    if (d === "install-guide") {
      const tool = st.tools.find(t => t.id === data.id) || st.tools[0];
      return `<div class="col" style="gap:10px">
        <div class="kv"><span class="k">工具</span><span>${tool.name} · ${tool.adapter}</span></div>
        <div class="col" style="gap:6px">
          ${[["1", "安装", "从官方渠道安装 " + tool.name + "（本原型不提供下载）。"],
             ["2", "登录", "在终端运行 " + tool.adapter + " 自己的登录命令，凭据由该 CLI 保存。"],
             ["3", "验证", "回到“本地工具”点击“测试认证”，成功后状态变为“已安装且可用”。"]]
            .map(([n, t, txt]) => `<div class="row" style="gap:9px;align-items:flex-start">
              <span class="ib" style="width:18px;height:18px;border-radius:50%;background:var(--bg-subtle);display:grid;place-items:center;font-size:10px;font-weight:600">${n}</span>
              <span class="col" style="gap:1px"><b class="small">${t}</b><span class="tiny muted" style="line-height:1.5">${txt}</span></span></div>`).join("")}
        </div>
        <div class="code-block">${tool.adapter} --version</div>
      </div>`;
    }
    if (d === "tool-error") {
      const tool = st.tools.find(t => t.id === data.id) || st.tools[0];
      return `<div class="code-block" style="white-space:pre-wrap">probe: ${tool.adapter}
exit: 124 (timeout 3000ms)
stderr: unable to read version metadata
hint: 该适配器可能不兼容当前系统；可手动指定路径重试。
state: undetectable（不会假定可用）</div>
        <div class="row" style="gap:8px">${BT({ label: "复制错误详情", variant: "secondary", size: "sm", action: "copy-text", data: { text: "probe " + tool.adapter + " exit 124 timeout", label: "错误详情" } })}
          ${BT({ label: "打开安装指南", variant: "ghost", size: "sm", action: "dialog-install-guide", data: { id: tool.id } })}</div>`;
    }
    if (d === "confirm-delete-role") {
      const a = st.agents.find(x => x.id === data.id) || st.agents[0];
      return `<div class="banner is-failure" style="align-items:flex-start">${I("alert", "sm")}
        <span class="small" style="flex:1 1 auto">将删除角色 <b>${esc(a.name)}</b>。该角色不能再被分配；历史 Run、审批记录与日志保留。</span></div>`;
    }
    if (d === "workspace-new") {
      return `<div class="field"><span class="lbl">名称</span><input class="input" id="nw-name" placeholder="例如：research-2026" /></div>
        <div class="field"><span class="lbl">本地目录</span><input class="input" id="nw-path" placeholder="~/work/research-2026" /></div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("lock", "sm")}<span class="tiny">写入范围默认关闭；禁止访问 ~/.ssh、~/.config 及其他工作空间。</span></div>`;
    }
    if (d === "ws-boundary") {
      const w = st.workspaces.find(x => x.id === st.wsId) || st.workspaces[0];
      return `<div class="field"><span class="lbl">可读取路径（逗号分隔）</span><input class="input" id="wb-read" value="${esc(w.read.join(", "))}" /></div>
        <div class="field"><span class="lbl">可写入路径（逗号分隔）</span><input class="input" id="wb-write" value="${esc(w.write.join(", "))}" /></div>
        <div class="banner is-blocked" style="align-items:flex-start">${I("alert", "sm")}<span class="tiny">禁止清单（~/.ssh、~/.config、其他工作空间）不能被覆盖；边界从下一个任务生效。</span></div>`;
    }
    if (d === "team-edit") {
      const team = st.teams.find(t => t.id === data.teamId) || st.teams[0];
      return `<div class="field"><span class="lbl">团队名称</span><input class="input" id="te-name" value="${esc(team.name)}" /></div>
        <div class="field"><span class="lbl">默认工作空间</span><input class="input" id="te-ws" value="${esc(team.ws)}" /></div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}<span class="tiny">主管：${team.supervisor} · 成员 ${team.members.length} 名。更改不会静默影响正在运行的 Run。</span></div>`;
    }
    if (d === "file-preview") {
      return `<div class="kv"><span class="k">路径</span><span class="mono">${esc(data.path)}</span></div>
        <div class="code-block" style="white-space:pre-wrap"># 对比表草稿（模拟内容）

| 维度 | 竞品 A | 竞品 B | 竞品 C |
|---|---|---|---|
| 定价 | ¥99/月 | ¥129/月 | — |
| 免费额度 | 有 | 无 | 有 |
| 数据来源 | S1 | S2 | — |

> 缺失单元格留空，不推测（按你的规则）。</div>`;
    }
    if (d === "file-path") {
      const w = st.workspaces.find(x => x.id === st.wsId) || st.workspaces[0];
      return `<div class="kv"><span class="k">工作空间</span><span>${w.name}</span></div>
        <div class="code-block">${esc(w.path)}</div>
        <div class="row" style="gap:8px">${BT({ label: "复制路径", variant: "secondary", size: "sm", action: "copy-path", data: { path: w.path } })}</div>
        <div class="tiny muted">真实应用会在此处调用系统文件管理器打开该目录。</div>`;
    }
    if (d === "shortcuts") {
      return `<div class="col" style="gap:8px">
        ${[["⌘K / Ctrl+K", "命令面板（↑↓ 选择，Enter 执行）"], ["Esc", "关闭菜单 / 命令面板 / 对话框 / 检查器"], ["Enter", "任务输入框发送消息（Shift+Enter 换行）"], ["Tab", "在侧栏、按钮与输入框之间移动"]]
          .map(([k, v]) => `<div class="kv"><span class="k mono">${k}</span><span style="flex:1 1 auto">${v}</span></div>`).join("")}
      </div>`;
    }
    if (d === "graph-new") {
      const team = st.teams.find(t => t.id === st.teamId) || st.teams[0];
      const parent = data.parent || team.supervisor;
      return `<div class="field"><span class="lbl">名称</span><input class="input" id="gn-name" placeholder="例如：事实核查员" /></div>
        <div class="field"><span class="lbl">职责描述</span><input class="input" id="gn-role" placeholder="例如：核对来源与数字，标注无法验证之处" /></div>
        <div class="grid-2">
          <div class="field"><span class="lbl">直属主管</span>
            <select class="select" id="gn-parent">${team.members.map(m => `<option value="${esc(m.name)}" ${m.name === parent ? "selected" : ""}>${m.name}${m.name === team.supervisor ? "（主管）" : ""}</option>`).join("")}<option value="用户">用户（成为团队主管）</option></select></div>
          <div class="field"><span class="lbl">执行工具</span>
            <select class="select" id="gn-tool">${st.tools.map(t => `<option value="${t.id}">${t.name}（${TOOL_STATE[t.state][1]}）</option>`).join("")}</select></div>
        </div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}<span class="tiny">保存只代表配置已保存；角色需要“已安装且可用”的工具才能真正执行任务。</span></div>`;
    }
    if (d === "graph-edit") {
      const team = st.teams.find(t => t.id === st.teamId) || st.teams[0];
      const m = team.members.find(x => x.name === data.name) || {};
      return `<div class="field"><span class="lbl">名称</span><input class="input" id="ge-name" value="${esc(m.name || "")}" /></div>
        <div class="field"><span class="lbl">职责描述</span><input class="input" id="ge-role" value="${esc(m.role || "")}" /></div>
        <div class="row" style="gap:8px"><span class="tiny muted">直属主管：${esc(m.reports || "—")}</span><span class="spacer"></span>
          ${BT({ label: "修改上级…", variant: "ghost", size: "sm", action: "graph-open-superior", data: { name: m.name } })}</div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}<span class="tiny">正在运行的 Run 保留原配置，改动从下一个新任务生效。</span></div>`;
    }
    if (d === "graph-superior") {
      const team = st.teams.find(t => t.id === st.teamId) || st.teams[0];
      const m = team.members.find(x => x.name === data.name) || {};
      const options = team.members.filter(x => x.name !== m.name).map(x => `<label class="row" style="gap:9px;padding:8px 10px;border:1px solid var(--border-default);border-radius:6px;cursor:pointer">
          <input type="radio" name="gs-sup" value="${esc(x.name)}" ${x.name === m.reports ? "checked" : ""} style="accent-color:var(--text-primary)" />
          <span class="col" style="gap:1px"><span class="small">${x.name}</span><span class="tiny muted">${x.name === team.supervisor ? "当前团队主管" : x.role}</span></span>
        </label>`).join("");
      return `<div class="col" style="gap:6px;max-height:280px;overflow:auto">${options}
          <label class="row" style="gap:9px;padding:8px 10px;border:1px solid var(--border-default);border-radius:6px;cursor:pointer">
            <input type="radio" name="gs-sup" value="用户" ${m.reports === "用户" ? "checked" : ""} style="accent-color:var(--text-primary)" />
            <span class="col" style="gap:1px"><span class="small">用户（直接汇报，成为团队主管）</span><span class="tiny muted">该成员将取代当前主管</span></span>
          </label></div>`;
    }
    if (d === "create-task") {
      return `<div class="field"><span class="lbl">任务描述</span>
          <textarea class="textarea" id="dlg-desc">${esc(st.composerText || "把 2025 Q3 三家竞品的公开资料整理成一份对比报告，标注数据来源；缺数据处留空并说明，不要推测。输出为 Markdown，并附一张对比表。")}</textarea></div>
        <div class="field"><span class="lbl">工作空间</span>
          <button class="select" data-action="menu-workspace"><span class="row" style="gap:8px">${I("folder", "sm")}<span class="mono" style="font-size:11.5px">${st.composerWs}</span></span>${I("down", "sm")}</button></div>
        <div class="banner is-blocked" style="align-items:flex-start">${I("alert", "sm")}
          <span class="tiny" style="line-height:1.5">检测到另一任务（Run #1244）正在写入 vocab-ops。本任务使用 ${st.composerWs}，当前无冲突；切换目录会提示文件冲突风险。</span></div>
        <div class="field"><span class="lbl">执行方式</span>
          <div class="mode-cards">
            ${[["快速执行", "一个 Agent 直接完成，不启动主管", "zap"], ["选择团队", "主管协调多个 Agent 分工与交接", "users"], ["自定义", "手动定义子任务、依赖与授权", "workflow"]]
              .map(([t, dd, ic]) => `<button class="mode-card ${st.composerMode === t ? "is-on" : ""}" data-action="set-mode" data-mode="${t}">
                <span class="row" style="gap:7px">${I(ic, "sm")}<b class="mt">${t}</b></span><span class="md">${dd}</span></button>`).join("")}
          </div></div>
        ${st.composerMode === "快速执行"
          ? `<div class="field"><span class="lbl">执行 Agent</span><button class="select" data-action="menu-dialog-agent"><span>${esc(st.dialogAgent || "研究员 · Claude Code")}</span>${I("down", "sm")}</button>
              <span class="hint">单 Agent 模式不会额外启动主管，输入区会标注目标 Agent。</span></div>`
          : st.composerMode === "选择团队"
          ? `<div class="field"><span class="lbl">执行团队</span><button class="select" data-action="menu-dialog-team"><span>${esc(st.dialogTeam || "调研团队 · 4 名成员 · 主管：研究协调员")}</span>${I("down", "sm")}</button></div>`
          : `<div class="field"><span class="lbl">子任务与依赖</span><div class="code-block">1 资料收集 → 2 对比表 → 3 结论 → 4 导出（依赖 3）</div>
              <span class="hint">高级模式需手动指定每一步的 Agent 与授权。</span></div>`}
        <div class="card panel" style="background:var(--bg-inset);gap:5px">
          <span class="lbl">本次任务的授权边界</span>
          ${[["check", "可读写 " + st.composerWs + "/ 及子目录；可创建新文件"], ["check", "可读取公开网页（仅研究 Agent，限 20 次请求）"], ["x", "不可执行删除或 git push；不可访问工作空间外的目录"]]
            .map(([ic, t]) => `<div class="row" style="gap:7px">${I(ic, "sm")}<span class="tiny secondary">${t}</span></div>`).join("")}
        </div>`;
    }
    if (d === "new-role") {
      return `<div class="field"><span class="lbl">名称</span><input class="input" id="nr-name" placeholder="例如：研究员" /></div>
        <div class="field"><span class="lbl">职责描述</span><input class="input" id="nr-desc" placeholder="例如：收集公开资料并记录来源编号" /></div>
        <div class="field"><span class="lbl">执行工具</span>
          <div class="segmented" style="flex-wrap:wrap">
            ${st.tools.map((t, i) => `<button class="${i === 0 ? "is-on" : ""}" data-action="new-role-tool" data-id="${t.id}" data-name="${t.name}">${t.name}（${TOOL_STATE[t.state][1]}）</button>`).join("")}
          </div>
          <span class="hint">选中的工具决定该角色能不能真正执行；未就绪时任务会停下并提示。</span></div>`;
    }
    if (d === "new-team") {
      return `<div class="field"><span class="lbl">团队名称</span><input class="input" id="nt-name" placeholder="例如：调研团队" /></div>
        <div class="field"><span class="lbl">主管 Agent</span><input class="input" id="nt-sup" value="研究协调员" /></div>
        <div class="banner is-neutral" style="align-items:flex-start">${I("info", "sm")}
          <span class="tiny">保存后可在团队页添加下属；若形成 A→B→A 的环会被阻止并提示原因。</span></div>`;
    }
    return "";
  }

  function dialogFooter(st) {
    const d = st.dialog;
    const map = {
      "create-task": [["取消", "close-dialog", "ghost"], ["创建任务", "create-task", "primary"]],
      "new-role": [["取消", "close-dialog", "ghost"], ["保存角色", "save-new-role", "primary"]],
      "new-team": [["取消", "close-dialog", "ghost"], ["保存团队", "save-team", "primary"]],
      templates: [["关闭", "close-dialog", "ghost"]],
      "tool-path": [["取消", "close-dialog", "ghost"], ["保存路径", "save-tool-path", "primary"]],
      "tool-config": [["关闭", "close-dialog", "ghost"]],
      "install-guide": [["关闭", "close-dialog", "ghost"], ["我已安装，重新扫描", "scan-tools", "primary"]],
      "tool-error": [["关闭", "close-dialog", "ghost"]],
      "confirm-delete-role": [["取消", "close-dialog", "ghost"], ["删除角色", "do-delete-role", "danger"]],
      "workspace-new": [["取消", "close-dialog", "ghost"], ["添加工作空间", "save-workspace", "primary"]],
      "ws-boundary": [["取消", "close-dialog", "ghost"], ["保存边界", "save-boundary", "primary"]],
      "team-edit": [["取消", "close-dialog", "ghost"], ["保存", "save-team-edit", "primary"]],
      "graph-new": [["取消", "close-dialog", "ghost"], ["创建并放入关系图", "graph-save-new", "primary"]],
      "graph-edit": [["取消", "close-dialog", "ghost"], ["保存", "graph-save-edit", "primary"]],
      "graph-superior": [["取消", "close-dialog", "ghost"], ["保存上级", "graph-save-superior", "primary"]],
      "file-preview": [["关闭", "close-dialog", "ghost"]],
      "file-path": [["关闭", "close-dialog", "ghost"]],
      shortcuts: [["关闭", "close-dialog", "ghost"]]
    };
    const items = map[d] || [["关闭", "close-dialog", "ghost"]];
    return `<span class="tiny muted">${d === "create-task" ? "创建任务 ≠ 开始执行：任务先进入排队" : "演示环境：不会写入真实系统"}</span><span class="spacer"></span>
      ${items.map(([label, action, variant]) => BT({ label, variant, action })).join("")}`;
  }

  S.overlays = function (st) {
    let html = "";

    if (st.menu) {
      html += `<div class="menu-pop" role="menu" style="left:${st.menu.x}px;top:${st.menu.y}px;width:${st.menu.width}px">
        ${st.menu.items.map(it => `<button class="menu-item ${it.checked ? "is-checked" : ""}" role="menuitem"
          data-action="${it.action}" ${Object.entries(it.data || {}).map(([k, v]) => `data-${k}="${esc(v)}"`).join(" ")}>
          <span class="mi-label">${it.label}</span>${it.checked ? I("check", "sm") : ""}</button>`).join("")}
      </div>`;
    }

    if (st.dialog) {
      const [title, sub] = DIALOG_TITLES[st.dialog] || ["对话框", ""];
      html += `<div class="scrim" data-action="close-dialog">
        <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="dlg-title" style="width:min(${st.dialog === "templates" || st.dialog === "shortcuts" ? 560 : 720}px,100%)" data-stop="1">
          <div class="dh">
            <div class="col" style="gap:4px">
              <b id="dlg-title" style="font-size:18px">${title}</b>
              <span class="tiny secondary">${sub}</span>
            </div>
            ${UI.iconBtn("x", "close-dialog", "关闭")}
          </div>
          <div class="db">${dialogBody(st)}</div>
          <div class="df">${dialogFooter(st)}</div>
        </div>
      </div>`;
    }

    if (st.palette) {
      const items = DATA.paletteCommands.filter(c => !st.paletteQuery || c.label.toLowerCase().includes(st.paletteQuery.toLowerCase()));
      html += `<div class="palette" data-action="close-palette">
        <div class="box" role="dialog" aria-label="命令面板" data-stop="1">
          <input id="palette-input" value="${esc(st.paletteQuery)}" placeholder="搜索页面或命令…（↑↓ 选择，Enter 执行）" aria-label="搜索页面或命令" />
          <div class="list" role="listbox">
            ${items.map((c, i) => `<button class="item ${i === st.paletteIndex ? "is-on" : ""}" role="option" aria-selected="${i === st.paletteIndex}"
              data-action="palette-run" data-index="${i}">${I("right", "sm muted")}${c.label}</button>`).join("") || `<div class="item muted">没有匹配项</div>`}
          </div>
        </div>
      </div>`;
    }

    if (st.toasts.length) {
      html += `<div class="toasts" aria-live="polite">${st.toasts.map(t => `<div class="toast ${t.kind ? "is-" + t.kind : ""}">
        ${I(t.kind === "success" ? "check" : t.kind === "failure" ? "x" : t.kind === "blocked" ? "alert" : "info", "sm")}
        <span>${t.text}</span></div>`).join("")}</div>`;
    }
    return html;
  };
})();
