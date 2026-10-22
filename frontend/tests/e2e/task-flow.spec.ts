import { expect, test } from "@playwright/test";

/** 任务核心链路：创建 → 排队 → 执行 → 暂停 → 继续 → 停止。 */
test("创建任务并完成暂停/继续/停止", async ({ page }) => {
  await page.goto("/#/dashboard");
  await page.getByRole("textbox", { name: "描述你要完成的工作" }).fill("把三家竞品的定价整理成对比表");
  await page.getByRole("button", { name: "创建任务" }).click();
  await expect(page.getByRole("combobox", { name: "执行团队" })).toBeVisible();
  await page.getByRole("button", { name: "创建任务" }).last().click();
  await expect(page).toHaveURL(/#\/tasks\//);
  await expect(page.getByText("排队中").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "暂停" })).toBeVisible({ timeout: 6000 });
  await page.getByRole("button", { name: "暂停" }).click();
  await expect(page.getByText("等待暂停").first()).toBeVisible();
  await expect(page.getByRole("button", { name: "继续" })).toBeVisible({ timeout: 6000 });
  await page.getByRole("button", { name: "继续" }).click();
  await expect(page.getByRole("button", { name: "停止" })).toBeVisible();
  await page.getByRole("button", { name: "停止" }).click();
  await expect(page.getByText("正在请求停止").first()).toBeVisible();
  await expect(page.getByText("已停止").first()).toBeVisible({ timeout: 6000 });
});

test("团队任务使用弹窗中选定的团队", async ({ page }) => {
  await page.goto("/#/tasks");
  await page.getByRole("button", { name: "新建任务" }).click();
  await page.getByRole("combobox", { name: "工作空间" }).click();
  await page.getByRole("option", { name: /release-2\.4/ }).click();
  await expect(page.getByRole("button", { name: "创建任务" })).toBeDisabled();
  await page.getByRole("combobox", { name: "执行团队" }).click();
  await page.getByRole("option", { name: "内容团队 · 2 人" }).click();
  await page.getByRole("textbox", { name: "任务描述" }).fill("验证团队绑定");
  await page.getByRole("button", { name: "创建任务" }).click();
  await expect(page).toHaveURL(/#\/tasks\//);
  await expect(page.getByText("内容团队 · 2 人").first()).toBeVisible();
});

test("单 Agent 任务使用选定的 Agent 且不创建主管", async ({ page }) => {
  await page.goto("/#/tasks");
  await page.getByRole("button", { name: "新建任务" }).click();
  await page.getByRole("button", { name: /快速执行/ }).click();
  const agent = page.getByRole("combobox", { name: "目标 Agent" });
  await expect(agent).toBeVisible();
  await agent.click();
  await page.getByRole("option", { name: /研究员 · Claude Code/ }).click();
  await page.getByRole("textbox", { name: "任务描述" }).fill("验证单 Agent 绑定");
  await page.getByRole("button", { name: "创建任务" }).click();
  await expect(page).toHaveURL(/#\/tasks\//);
  await expect(page.getByText("研究员 · 单 Agent").first()).toBeVisible();
  await expect(page.getByRole("textbox", { name: "向目标 Agent 发送消息" })).toBeVisible();
});

test("自定义任务保持待配置占位且不显示调度或对话入口", async ({ page }) => {
  await page.goto("/#/tasks");
  await page.getByRole("button", { name: "新建任务" }).click();
  await page.getByRole("button", { name: /自定义执行/ }).click();
  await expect(page.getByText("计划编辑尚未实现；此模式只会创建待配置占位任务，不会开始执行。", { exact: true })).toBeVisible();
  await page.getByRole("textbox", { name: "任务描述" }).fill("配置自定义执行计划");
  await page.getByRole("button", { name: "创建任务" }).click();

  await expect(page).toHaveURL(/#\/tasks\//);
  await expect(page.getByText("待配置", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("此任务是待配置占位，不会被调度。配置自定义子任务和依赖后，才能开始执行。", { exact: true })).toBeVisible();
  await expect(page.getByText("执行计划尚未配置", { exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "向主管发送消息" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "暂停" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "继续" })).toHaveCount(0);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  const { width, height } = page.viewportSize()!;
  await page.screenshot({ path: `test-results/visual/${width}x${height}/light-custom-placeholder.png` });
});
