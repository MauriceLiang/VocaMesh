import { expect, test } from "@playwright/test";

/** 任务核心链路：创建 → 排队 → 执行 → 暂停 → 继续 → 停止。 */
test("创建任务并完成暂停/继续/停止", async ({ page }) => {
  await page.goto("/#/dashboard");
  await page.getByRole("textbox", { name: "描述你要完成的工作" }).fill("把三家竞品的定价整理成对比表");
  await page.getByRole("button", { name: "创建任务" }).click();
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
