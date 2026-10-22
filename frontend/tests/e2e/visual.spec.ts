import { expect, test } from "@playwright/test";

/**
 * 视觉对照：按页面与主题截图到 test-results/visual，与 UI/qa/evidence/*.png 并排比对。
 * 允许：字体栅格化差异。不允许：间距/字号/圆角/颜色/溢出差异。
 */
const pages: Array<[string, string]> = [
  ["dashboard", "/#/dashboard"],
  ["tasks", "/#/tasks"],
  ["task-detail", "/#/tasks/t1"],
  ["tools", "/#/tools"],
  ["agents", "/#/agents"],
  ["teams", "/#/teams"],
  ["approvals", "/#/approvals"],
  ["history", "/#/history"],
  ["settings", "/#/settings"],
  ["notifications", "/#/notifications"],
  ["help", "/#/help"],
];

for (const theme of ["light", "dark"] as const) {
  for (const [name, url] of pages) {
    test(`visual ${theme} ${name}`, async ({ page }) => {
      await page.goto(url);
      if (theme === "dark") await page.locator(".v-toolbar").getByRole("button", { name: "切换到暗色" }).click();
      await page.waitForTimeout(300);
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow).toBeLessThanOrEqual(0);
      await expect(page.locator(".v-content")).toBeVisible();
      const { width, height } = page.viewportSize()!;
      await page.screenshot({ path: `test-results/visual/${width}x${height}/${theme}-${name}.png` });
    });
  }

  test(`visual ${theme} create-task`, async ({ page }) => {
    await page.goto("/#/dashboard");
    if (theme === "dark") await page.locator(".v-toolbar").getByRole("button", { name: "切换到暗色" }).click();
    await page.getByRole("button", { name: "新建任务" }).click();
    await expect(page.getByRole("combobox", { name: "执行团队" })).toBeVisible();
    const { width, height } = page.viewportSize()!;
    await page.screenshot({ path: `test-results/visual/${width}x${height}/${theme}-create-task.png` });
  });
}
