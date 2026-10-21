/* VocaMesh prototype · mock data
   All names, paths, Run ids and log lines are fictional. No CLI, API or filesystem is touched. */

window.DATA = {
  nav: [
    { group: "工作", items: [
      { screen: "dashboard", label: "工作台", icon: "dashboard" },
      { screen: "tasks", label: "全部任务", icon: "list" },
      { screen: "approvals", label: "待审批", icon: "shield", count: "approvals" },
      { screen: "history", label: "历史记录", icon: "history" }
    ]},
    { group: "Agent 与协作", items: [
      { screen: "roles", label: "Agent 角色", icon: "bot" },
      { screen: "tools", label: "本地工具", icon: "terminal" },
      { screen: "teams", label: "Agent 团队", icon: "branch" },
      { screen: "workspaces", label: "工作空间", icon: "folder" }
    ]},
    { group: "应用", items: [
      { screen: "notifications", label: "通知", icon: "bell", count: "notifications" }
    ]}
  ],
  footNav: [
    { screen: "settings", label: "设置", icon: "settings" },
    { screen: "help", label: "帮助与反馈", icon: "help" }
  ],

  tasks: [
    {
      id: "t1",
      name: "整理 Q3 竞品调研并生成对比报告",
      ws: "research-2025",
      team: "调研团队 · 4 人",
      run: "#1247",
      supervisor: "研究协调员",
      started: "14:02",
      state: "running",
      stateNote: "已运行 12 分钟",
      subtasks: [
        { id: "s1", name: "拆解任务并分配角色", agent: "研究协调员 · 主管", state: "success", result: "已生成 4 个子任务与依赖" },
        { id: "s2", name: "收集三家竞品的公开资料", agent: "研究员 · Claude Code", state: "success", result: "12 个来源，含 3 个官方定价页" },
        { id: "s3", name: "生成对比表格草稿", agent: "分析师 · Codex CLI", state: "running", result: "正在写入 docs/compare-draft.md" },
        { id: "s4", name: "撰写结论与风险提示", agent: "写作者 · Claude Code", state: "waiting-dependency", result: "依赖 #3 完成后自动开始" },
        { id: "s5", name: "导出报告并写入工作空间", agent: "写作者 · Claude Code", state: "waiting-approval", result: "需要一次性写入授权" }
      ],
      messages: [
        { who: "supervisor", name: "研究协调员 · 主管", time: "14:02", text: "已把任务拆成 4 个阶段：资料收集 → 对比表 → 结论 → 导出。资料收集已完成，对比表正在生成。第 4 步写入文件需要你的授权，我不会代替你同意。" },
        { who: "me", name: "你", time: "14:06", text: "优先保证来源可追溯。缺数据的地方留空并说明，不要推测。" }
      ],
      instruction: null
    },
    {
      id: "t2",
      name: "批量规范化 320 条术语词表",
      ws: "vocab-ops",
      team: "数据小组 · 2 人",
      run: "#1244",
      supervisor: "数据协调员",
      started: "12:55",
      state: "waiting-approval",
      stateNote: "1 项写入等待授权",
      subtasks: [
        { id: "s1", name: "读取 terms/ 全部词条", agent: "数据 Agent", state: "success", result: "320 条 · 5 个文件" },
        { id: "s2", name: "规范化并覆盖写回", agent: "数据 Agent", state: "waiting-approval", result: "需要写入授权" }
      ],
      messages: [
        { who: "supervisor", name: "数据协调员 · 主管", time: "13:10", text: "写入需要你确认。该目录同时被另一个任务使用，我已暂停派发新工作。" }
      ],
      instruction: null
    },
    {
      id: "t3",
      name: "为更新日志生成中英文双版本文案",
      ws: "release-2.4",
      team: "内容团队 · 3 人",
      run: "#1241",
      supervisor: "内容主管",
      started: "13:48",
      state: "running",
      stateNote: "3/5 子任务完成",
      subtasks: [
        { id: "s1", name: "提取本次变更清单", agent: "内容主管", state: "success", result: "14 条变更" },
        { id: "s2", name: "生成中文初稿", agent: "写作者", state: "success", result: "docs/changelog-zh.md" },
        { id: "s3", name: "生成英文初稿", agent: "写作者", state: "success", result: "docs/changelog-en.md" },
        { id: "s4", name: "校对术语一致性", agent: "校对 Agent", state: "running", result: "正在比较 38 个术语" },
        { id: "s5", name: "合并与导出", agent: "内容主管", state: "waiting-dependency", result: "依赖 #4" }
      ],
      messages: [
        { who: "supervisor", name: "内容主管 · 主管", time: "13:50", text: "两个语言版本已生成，正在校对术语；校对完成后我会合并。" }
      ],
      instruction: null
    },
    {
      id: "t4",
      name: "重构 CLI 适配器的错误处理",
      ws: "core-dev",
      team: "开发者团队 · 2 人",
      run: "#1238",
      supervisor: "开发主管",
      started: "13:20",
      state: "queued",
      stateNote: "排队等待资源（并发 3/3）",
      subtasks: [
        { id: "s1", name: "阅读现有限流与重试逻辑", agent: "开发 Agent", state: "pending", result: "未开始" },
        { id: "s2", name: "重写错误分类", agent: "开发 Agent", state: "pending", result: "未开始" }
      ],
      messages: [
        { who: "supervisor", name: "开发主管 · 主管", time: "13:21", text: "当前并发已满，任务在队列中。不会丢弃，也不会假装已开始。" }
      ],
      instruction: null
    },
    {
      id: "t5",
      name: "整理供应商合同条款对比",
      ws: "legal-docs",
      team: "法务助手 · 单 Agent",
      run: "#1245",
      supervisor: "法务助手",
      started: "13:20",
      state: "failed",
      stateNote: "读取 PDF 时异常退出",
      subtasks: [
        { id: "s1", name: "读取 6 份合同 PDF", agent: "法务助手", state: "failure", result: "PDF 编码无法解析，进程退出（exit 1）" },
        { id: "s2", name: "提取条款并对比", agent: "法务助手", state: "blocked", result: "依赖 #1 失败，已阻塞" }
      ],
      messages: [
        { who: "supervisor", name: "法务助手", time: "13:22", text: "读取第 3 份合同时进程退出。我停下等待你的选择：只重试失败部分，或跳过并生成标注完整性的部分成果。" }
      ],
      instruction: null
    },
    {
      id: "t6",
      name: "导出词汇覆盖率报告.csv",
      ws: "vocab-ops",
      team: "数据小组 · 2 人",
      run: "#1242",
      supervisor: "数据协调员",
      started: "11:58",
      state: "done",
      stateNote: "产物已保存到 exports/",
      subtasks: [
        { id: "s1", name: "统计覆盖率", agent: "数据 Agent", state: "success", result: "覆盖率 87.4%" },
        { id: "s2", name: "导出 CSV", agent: "数据 Agent", state: "success", result: "exports/coverage-2025q3.csv" }
      ],
      messages: [{ who: "supervisor", name: "数据协调员 · 主管", time: "12:02", text: "报告已导出，覆盖率 87.4%。" }],
      instruction: null
    }
  ],

  approvals: [
    {
      id: "a1", taskId: "t1", state: "pending", age: "2 分钟前",
      title: "写入文件 /docs/report-q3.md", agent: "写作者 · Claude Code",
      target: "write  /docs/report-q3.md   (覆盖已存在文件 · 2.4 KB → 6.1 KB)",
      diff: "+ 新增 42 行   ~ 修改 6 行   - 删除 0 行",
      scope: "单个文件 · 单次 · 仅本 Run。不会扩展到其他文件或其他任务。",
      risk: [
        "目标文件已存在，写入会覆盖原有内容",
        "该文件不在本任务的默认写入白名单内",
        "若拒绝，Agent 会停在当前步骤，不会自动重试"
      ],
      history: [
        ["14:04:02", "请求已发出", "写作者 · Run #1247 · 已送达运行时"],
        ["14:04:02", "任务已暂停派发", "等待用户决定；其他无依赖子任务不受影响"],
        ["现在", "等待你的决定", "超时不会自动批准；2 小时后标记为已过期"]
      ]
    },
    {
      id: "a2", taskId: "t4", state: "pending", age: "18 分钟前",
      title: "执行命令 git push --force", agent: "开发 Agent · Codex CLI",
      target: "shell  git push --force origin main",
      diff: "重写远端 main 分支历史（3 个提交）",
      scope: "单条命令 · 单次 · 不可撤销。远端历史重写后无法自动回滚。",
      risk: ["会重写远端分支历史，其他人已拉取的提交会冲突", "该命令不在默认白名单内", "拒绝后任务停在当前步骤，不会重试"],
      history: [
        ["13:46:10", "请求已发出", "开发 Agent · Run #1238"],
        ["13:46:10", "任务已暂停派发", "等待用户决定"],
        ["现在", "等待你的决定", "超过 2 小时将标记为已过期"]
      ]
    },
    {
      id: "a3", taskId: "t2", state: "expired", age: "2 小时前",
      title: "删除目录 terms/archive/", agent: "数据 Agent",
      target: "delete  terms/archive/   (38 个文件)",
      diff: "删除 38 个文件 · 约 1.2 MB",
      scope: "目录 · 单次 · 不可撤销",
      risk: ["删除操作不可撤销", "目录内包含历史版本"],
      history: [["11:20:04", "请求已发出", "数据 Agent"], ["13:20:04", "已过期", "超时未处理；不会自动批准"]]
    },
    {
      id: "a4", taskId: "t1", state: "denied", age: "1 小时前",
      title: "访问网络：竞品定价页", agent: "研究员 · Claude Code",
      target: "network  GET https://example-competitor.test/pricing",
      diff: "1 次请求 · 目标为公开页面",
      scope: "单次 · 仅研究 Agent",
      risk: ["会把内部术语表片段发送到外部页面参数"],
      history: [["13:02:11", "请求已发出", "研究员"], ["13:03:02", "已拒绝（你）", "原因：不向外部发送内部术语"]]
    }
  ],

  tools: [
    { id: "claude", name: "Claude Code", adapter: "claude-code", version: "v2.1.4", path: "/usr/local/bin/claude", state: "ready", note: "已验证登录 · 3 个角色在用" },
    { id: "codex", name: "Codex CLI", adapter: "codex-cli", version: "v0.9.2", path: "/opt/homebrew/bin/codex", state: "auth-unknown", note: "检测到配置文件，但无法确认登录状态" },
    { id: "opencode", name: "OpenCode", adapter: "opencode", version: "—", path: "未检测到", state: "missing", note: "需要手动安装后才能被角色使用" },
    { id: "omni", name: "Omni Adapter", adapter: "omni", version: "v1.2.0", path: "~/.local/bin/omni", state: "undetectable", note: "版本读取超时，可能不兼容当前系统" }
  ],

  agents: [
    { id: "r1", name: "研究协调员", role: "主管 · 向用户汇报", tool: "Claude Code · v2.1.4", state: "ready", desc: "作为主管：拆解任务、分配子任务、汇总结论并向我汇报。遇到需要授权或可能影响计划的变更时，先停下询问。", instr: "1. 先确认目标与验收标准，再拆分阶段。\n2. 数据必须可追溯：每个结论附来源编号，缺数据留空。\n3. 不代替用户批准高风险操作；不在未确认时重试已失败的执行。", perms: { read: true, write: true, shell: false, net: true } },
    { id: "r2", name: "研究员", role: "Claude Code", tool: "Claude Code · v2.1.4", state: "ready", desc: "收集公开资料并记录来源编号，不推测缺失数据。", instr: "为每个结论附来源编号；无法确认的信息标注为“未验证”。", perms: { read: true, write: false, shell: false, net: true } },
    { id: "r3", name: "分析师", role: "Codex CLI", tool: "Codex CLI · v0.9.2", state: "auth-unknown", desc: "生成对比表格与结构化草稿。", instr: "输出 Markdown 表格；缺失单元格留空并在备注列说明。", perms: { read: true, write: true, shell: false, net: false } },
    { id: "r4", name: "写作者", role: "Claude Code", tool: "Claude Code · v2.1.4", state: "not-ready", desc: "撰写结论、整理措辞并导出。", instr: "结论必须引用表格中的行号；不使用未经确认的数字。", perms: { read: true, write: true, shell: false, net: false } },
    { id: "r5", name: "法务助手", role: "本地模型（实验）", tool: "本地模型 · 实验", state: "unknown", desc: "读取合同并提取条款。", instr: "只做提取与对比，不提供法律意见。", perms: { read: true, write: false, shell: false, net: false } }
  ],

  teams: [
    { id: "tm1", name: "调研团队", supervisor: "研究协调员", ws: "research-2025", members: [
      { name: "研究协调员", role: "主管 · 向用户汇报", reports: "用户", tool: "Claude Code", state: "ready" },
      { name: "研究员", role: "资料收集", reports: "研究协调员", tool: "Claude Code", state: "ready" },
      { name: "分析师", role: "表格与结构", reports: "研究协调员", tool: "Codex CLI", state: "auth-unknown" },
      { name: "写作者", role: "结论与导出", reports: "研究协调员", tool: "Claude Code", state: "not-ready" }
    ]},
    { id: "tm2", name: "内容团队", supervisor: "内容主管", ws: "release-2.4", members: [
      { name: "内容主管", role: "主管", reports: "用户", tool: "Claude Code", state: "ready" },
      { name: "写作者", role: "撰写", reports: "内容主管", tool: "Claude Code", state: "ready" },
      { name: "校对 Agent", role: "术语校对", reports: "内容主管", tool: "Claude Code", state: "ready" }
    ]},
    { id: "tm3", name: "开发者团队", supervisor: "开发主管", ws: "core-dev", members: [
      { name: "开发主管", role: "主管", reports: "用户", tool: "Claude Code", state: "ready" },
      { name: "开发 Agent", role: "实现", reports: "开发主管", tool: "Codex CLI", state: "auth-unknown" }
    ]}
  ],

  workspaces: [
    { id: "w1", name: "research-2025", path: "~/work/research-2025", tasks: 3, state: "ready", read: ["terms/", "docs/", "exports/"], write: ["terms/", "exports/"], deny: ["~/.ssh", "~/.config", "其他工作空间"] },
    { id: "w2", name: "vocab-ops", path: "~/work/vocab-ops", tasks: 5, state: "conflict", read: ["terms/", "docs/", "exports/"], write: ["terms/", "exports/"], deny: ["~/.ssh", "~/.config", "其他工作空间"] },
    { id: "w3", name: "release-2.4", path: "~/work/release-2.4", tasks: 2, state: "ready", read: ["docs/", "changelog/"], write: ["docs/"], deny: ["src/", "~/.ssh"] },
    { id: "w4", name: "legal-docs", path: "~/work/legal-docs", tasks: 1, state: "readonly", read: ["contracts/"], write: [], deny: ["全部写入操作"] }
  ],

  runs: [
    { id: "#1247", task: "整理 Q3 竞品调研并生成对比报告", who: "调研团队", state: "running", start: "14:02", dur: "12m" },
    { id: "#1246", task: "为更新日志生成中英文双版本文案", who: "内容团队", state: "success", start: "13:48", dur: "6m" },
    { id: "#1245", task: "整理供应商合同条款对比", who: "法务助手", state: "failure", start: "13:20", dur: "2m" },
    { id: "#1244", task: "批量规范化 320 条术语词表", who: "数据小组", state: "partial", start: "12:55", dur: "9m" },
    { id: "#1243", task: "统计术语覆盖率", who: "数据小组", state: "stopped", start: "12:30", dur: "4m" },
    { id: "#1242", task: "导出词汇覆盖率报告.csv", who: "数据小组", state: "unknown", start: "11:58", dur: "—" },
    { id: "#1241", task: "整理周会纪要与待办", who: "内容团队", state: "success", start: "昨天", dur: "3m" }
  ],

  partialRun: {
    id: "#1244",
    groups: [
      { kind: "success", label: "已完成", items: [
        ["terms/a-in.md", "规范化完成"],
        ["terms/j-r.md", "规范化完成"],
        ["terms/s-z.md", "规范化完成"]
      ]},
      { kind: "neutral", label: "跳过", items: [["terms/archive/", "等待授权超时，未处理"]] },
      { kind: "failure", label: "失败", items: [["terms/import-raw.csv", "编码无法识别，已保留原文件"]] }
    ]
  },

  notifications: [
    { id: "n1", kind: "blocked", title: "审批请求：写入 /docs/report-q3.md", body: "写作者 正在等待你的决定 · 任务 12 分钟后仍处于等待状态", time: "2 分钟前", screen: "approvals", unread: true },
    { id: "n2", kind: "failure", title: "任务失败：整理供应商合同条款对比", body: "法务助手 在读取 PDF 时异常退出（Run #1245）", time: "25 分钟前", screen: "task", taskId: "t5", unread: true },
    { id: "n3", kind: "neutral", title: "已暂停：统计术语覆盖率", body: "在安全检查点暂停，已有 2 项成果保留", time: "1 小时前", screen: "task", taskId: "t2", unread: true },
    { id: "n4", kind: "success", title: "任务完成：导出词汇覆盖率报告.csv", body: "数据小组 · Run #1242 · 产物已保存到 exports/", time: "今天 15:41", screen: "history", unread: false },
    { id: "n5", kind: "blocked", title: "审批已过期：删除目录 terms/archive/", body: "超过 2 小时未处理，已标记为过期；不会自动批准", time: "2 小时前", screen: "approvals", unread: false }
  ],

  logs: [
    ["14:02:07", "INFO", "supervisor", "plan created · 4 subtasks · deps=[[1],[2],[3]]"],
    ["14:02:09", "INFO", "researcher", "adapter=claude-code status=ready workdir=~/work/research-2025"],
    ["14:02:14", "INFO", "researcher", "read 6 sources · 3 official pricing pages"],
    ["14:02:31", "WARN", "researcher", "source#3 http 403 · retry 1/1 after 20s"],
    ["14:03:02", "INFO", "analyst", "adapter=codex-cli status=running model=default"],
    ["14:03:04", "INFO", "analyst", "write docs/compare-draft.md · 2.4 KB"],
    ["14:03:18", "WARN", "analyst", "table row 7 missing price_cny · left blank per user rule"],
    ["14:03:40", "INFO", "analyst", "diff +38 / -0 lines"],
    ["14:03:55", "INFO", "writer", "waiting dependency: node#3"],
    ["14:04:02", "INFO", "writer", "approval requested · write /docs/report-q3.md"],
    ["14:04:02", "INFO", "system", "task state → waiting_approval · dispatch paused"],
    ["14:04:15", "INFO", "supervisor", "notified user · 1 decision pending"],
    ["14:04:22", "DEBUG", "adapter", "heartbeat ok · queue=1 · concurrency 2/3"],
    ["14:04:40", "INFO", "analyst", "stream idle 38s · no output"]
  ],

  paletteCommands: [
    { label: "新建任务", screen: "dashboard", action: "create-task" },
    { label: "全部任务", screen: "tasks" },
    { label: "待审批", screen: "approvals" },
    { label: "历史记录", screen: "history" },
    { label: "本地工具", screen: "tools" },
    { label: "Agent 角色", screen: "roles" },
    { label: "Agent 团队", screen: "teams" },
    { label: "工作空间", screen: "workspaces" },
    { label: "设置", screen: "settings" },
    { label: "切换亮/暗主题", action: "toggle-theme" },
    { label: "切换界面密度", action: "toggle-density" },
    { label: "重置演示数据", action: "reset-demo" }
  ]
};
