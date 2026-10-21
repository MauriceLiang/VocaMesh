import { expect, test } from "@playwright/test";

/**
 * 视觉对照：按页面与主题截图到 test-results/visual，与 UI/qa/evidence/*.png 并排比对。
 * 允许：字体栅格化差异。不允许：间距/字号/圆角/颜色/溢出差异。
 */
const pages: Array<[string, string]> = [
  ["dashboard", "/#/dashboard"],
  ["tasks", "/#/tasks"],
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
      await page.screenshot({ path: `test-results/visual/${theme}-${name}.png` });
    });
  }
}
