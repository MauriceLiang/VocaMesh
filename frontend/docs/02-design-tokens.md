# 设计 Token 与还原对照

Token 唯一来源：`src/styles/tokens.css`（与 `UI/prototype/styles.css` 数值一致）。
Tailwind 通过 `@theme inline` 映射到工具类（`bg-surface`、`text-ink` 等），但组件样式使用 `.v-*` 类以保证像素级一致。

## 颜色

| Token | Light | Dark |
|---|---|---|
| `--bg-canvas` | `#F7F7F7` | `#101010` |
| `--bg-surface` | `#FFFFFF` | `#191919` |
| `--bg-subtle` | `#F1F1F1` | `#232323` |
| `--bg-selected` | `#FFFFFF` | `#292929` |
| `--bg-inset` | `#FAFAFA` | `#1F1F1F` |
| `--text-primary` | `#171717` | `#F4F4F4` |
| `--text-secondary` | `#5A5A5A` | `#B9B9B9` |
| `--text-muted` | `#6D6D6D` | `#A0A0A0` |
| `--border-default` | `#DEDEDE` | `#383838` |
| `--border-strong` | `#BABABA` | `#666666` |
| `--btn-primary-bg` | `#1B1B1B` | `#F3F3F3` |
| `--focus-outline` | `#4B4B4B` | `#E2E2E2` |
| `--status-success` / `-bg` | `#187447` / `#EAF4EE` | `#76D2A0` / `#14261D` |
| `--status-failure` / `-bg` | `#B13A43` / `#FBECED` | `#FF9299` / `#2A1719` |
| `--status-blocked` / `-bg` | `#8B5A00` / `#FBF2E2` | `#EBC16B` / `#2A2114` |
| `--status-neutral` / `-bg` | `#666666` / `#F1F1F1` | `#B5B5B5` / `#232323` |

## 尺寸

| 项 | 值 |
|---|---|
| 页面内边距 | 26（compact 20） |
| 列表列 / 右栏 / 分区导航 | 320 / 340 / 200 |
| 列间距 | 12 |
| 卡片内边距 | 列表 12 · 详情面板 16 · 导航容器 8 |
| 控件高 | 32（compact 28）；`sm` 按钮 26 |
| 行高 | 34（compact 30） |
| 圆角 | badge 4 · 控件 6 · 卡片 8 · 对话框 10 |
| 字号 | 正文 13 · 辅助 11.5 · 元信息 10.5 · 页面标题 22 |
| 侧栏 / 检查器 | 240 固定 / 360（≤1080 变抽屉 320） |

## 阴影

```css
--shadow-selected: 0 1px 3px rgb(0 0 0 / 10%), 0 4px 10px rgb(0 0 0 / 5%);
--shadow-overlay:  0 8px 24px rgb(0 0 0 / 12%);
--shadow-dialog:   0 16px 48px rgb(0 0 0 / 18%);
/* 暗色：48%/25%、45%、55% */
```

## 状态 → 语义映射（`lib/status.ts`）

| 对象 | 状态 | tone | 图标 |
|---|---|---|---|
| 任务 | 已完成 | success | check |
| 任务 | 执行失败 | failure | x |
| 任务 | 等待审批 / 需要处理 / 部分完成 | blocked | alert |
| 任务 | 执行中 / 排队 / 已暂停 | neutral | loader / hourglass / pause |
| 子任务 | 成功 / 失败 / 阻塞 / 等待依赖 | success / failure / blocked / neutral | — |
| 工具 | 已安装且可用 / 认证待验证 / 无法检测 | success / blocked / unknown | — |
| 审批 | 等待审批 / 已允许 / 已拒绝 / 已过期 | blocked / success / failure / neutral | — |

## 主题与密度

`<html data-theme="light|dark" data-density="comfortable|compact" data-platform="mac|windows">`；
由 `stores/app.ts` 的 `watchEffect` 同步，与原型完全一致。
