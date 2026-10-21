# composables/

预留：跨 feature 复用的组合式函数。

当前可提取的候选（尚未抽取，避免过早抽象）：
- `useTransientMessage`（Toast 去重与计时，现位于 `stores/app.ts`）
- `useTaskAction`（暂停/停止/重试的确认与提示，现分散在 TaskDetailView）
- `useFilteredList`（列表筛选，现位于各 Store 的 computed）

原则：同一逻辑出现第三个使用点时再抽取。
