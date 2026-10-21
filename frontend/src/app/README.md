# app/

预留：应用级初始化（启动流程、错误边界、全局 Provider）。

当前实现：
- 启动逻辑在 `src/main.ts`（先加载设置，再挂载）。
- 外壳在 `src/components/layout/AppShell.vue`。

迁移建议：当需要接入 Tauri 事件桥、全局快捷键、崩溃兜底时，把对应逻辑集中到此目录。
