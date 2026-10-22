import { expect, test } from "@playwright/test";

/** 关系图：默认视图、缩放、节点菜单、视图切换只保留一处。 */
test("关系图可缩放且只有一个视图切换", async ({ page }) => {
  await page.goto("/#/teams");
  await expect(page.locator(".vue-flow")).toBeVisible();
  await expect(page.locator(".v-segmented")).toHaveCount(1);
  await expect(page.locator(".vue-flow__node")).toHaveCount(4);
  const viewport = await page.locator(".vue-flow").boundingBox();
  expect(viewport).not.toBeNull();
  for (const node of await page.locator(".vue-flow__node").all()) {
    const box = await node.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(viewport!.x);
    expect(box!.y).toBeGreaterThanOrEqual(viewport!.y);
    expect(box!.x + box!.width).toBeLessThanOrEqual(viewport!.x + viewport!.width);
    expect(box!.y + box!.height).toBeLessThanOrEqual(viewport!.y + viewport!.height);
  }
  const before = await page.locator(".vue-flow__transformationpane").getAttribute("style");
  await page.getByRole("button", { name: "放大" }).click();
  const after = await page.locator(".vue-flow__transformationpane").getAttribute("style");
  expect(after).not.toBe(before);
});

test("节点菜单包含编辑/加下属/改上级/设为主管/移除", async ({ page }) => {
  await page.goto("/#/teams");
  await expect(page.locator(".vue-flow__node")).toHaveCount(4);
  await page.getByRole("button", { name: "适应视图" }).last().click();
  const graph = await page.locator(".vue-flow").boundingBox();
  expect(graph?.height).toBeGreaterThanOrEqual(220);
  const node = page.locator(".vue-flow__node").last();
  await node.scrollIntoViewIfNeeded();
  await node.click();
  await node.getByRole("button", { name: /更多操作/ }).click();
  for (const label of ["编辑 Agent 信息", "添加下属", "添加直属上级…", "设为团队主管", "从此团队移除"]) {
    await expect(page.getByRole("button", { name: label, exact: true })).toBeVisible();
  }
});
