import { expect, test } from "@playwright/test";

test("状态、时间与搜索筛选可组合，清除后恢复任务列表", async ({ page }) => {
  await page.goto("/#/tasks");
  const rows = page.locator(".v-table tbody tr");

  await page.getByRole("button", { name: /筛选：/ }).click();
  await page.getByRole("menuitem", { name: "等待审批" }).click();
  await expect(rows.getByText("批量规范化 320 条术语词表")).toBeVisible();
  await expect(rows.getByText("整理 Q3 竞品调研并生成对比报告")).toHaveCount(0);

  await page.getByRole("button", { name: /时间：/ }).click();
  await page.getByRole("menuitem", { name: "最近 7 天" }).click();
  await expect(rows.getByText("批量规范化 320 条术语词表")).toBeVisible();

  await page.getByRole("textbox", { name: "搜索任务" }).fill("does-not-exist");
  await expect(page.getByText("没有匹配的任务")).toBeVisible();
  await page.getByRole("button", { name: "清除筛选" }).click();
  await expect(rows.getByText("整理 Q3 竞品调研并生成对比报告")).toBeVisible();
  await expect(page.getByRole("button", { name: "筛选：全部" })).toBeVisible();
  await expect(page.getByRole("button", { name: "时间：全部时间" })).toBeVisible();
});
