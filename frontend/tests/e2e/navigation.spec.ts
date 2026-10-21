import { expect, test } from "@playwright/test";

/** 导航与外壳：侧栏固定展开（无折叠按钮）、11 个一级入口、命令面板。 */
test("侧栏固定展开且没有折叠按钮", async ({ page }) => {
  await page.goto("/#/dashboard");
  const sidebar = page.locator(".v-sidebar");
  await expect(sidebar).toBeVisible();
  await expect(sidebar).toHaveCSS("width", "240px");
  await expect(page.getByRole("button", { name: /折叠侧栏|展开侧栏/ })).toHaveCount(0);
});

test("一级入口全部可访问", async ({ page }) => {
  await page.goto("/#/dashboard");
  const entries = ["工作台", "全部任务", "待审批", "历史记录", "Agent 角色", "本地工具", "Agent 团队", "工作空间", "通知", "设置", "帮助与反馈"];
  for (const label of entries) {
    await page.locator(".v-sidebar").getByText(label, { exact: true }).click();
    await expect(page.locator(".v-page-title")).toBeVisible();
  }
});

test("命令面板 ⌘K 可跳转", async ({ page }) => {
  await page.goto("/#/dashboard");
  await page.keyboard.press("Meta+K");
  const input = page.getByPlaceholder("搜索页面或命令…");
  await expect(input).toBeVisible();
  await input.fill("历史");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/#\/history/);
});
