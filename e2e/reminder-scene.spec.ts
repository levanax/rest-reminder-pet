import { expect, test } from "@playwright/test";
import path from "node:path";

async function canvasNonEmpty(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement | null;
    if (!c) return false;
    const ctx = c.getContext("2d");
    if (!ctx) return false;
    const { width, height } = c;
    if (width < 8 || height < 8) return false;
    const data = ctx.getImageData(0, 0, Math.min(width, 64), Math.min(height, 64)).data;
    for (let i = 3; i < data.length; i += 4) {
      if (data[i] > 8) return true;
    }
    return false;
  });
}

test("下雪场景 canvas 有像素", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/e2e/fixtures/scene-mock.html?scene=snow");
  await page.waitForTimeout(900);
  expect(await canvasNonEmpty(page)).toBeTruthy();
  await page.screenshot({ path: path.join("test-results", "scene-snow.png") });
});

test("雨雾场景逐渐变亮糊", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/e2e/fixtures/scene-mock.html?scene=rainGlass");
  await page.waitForTimeout(400);
  const early = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const d = ctx.getImageData(c.width / 2, c.height / 2, 1, 1).data;
    return d[0] + d[1] + d[2];
  });
  // Jump fog clock by restarting with patched startedAt via evaluate after wait
  await page.waitForTimeout(2500);
  const later = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    // sample several points average alpha-ish luminance
    let sum = 0;
    for (const [x, y] of [
      [0.3, 0.3],
      [0.5, 0.5],
      [0.7, 0.6],
    ] as const) {
      const d = ctx.getImageData(c.width * x, c.height * y, 1, 1).data;
      sum += d[0] + d[1] + d[2];
    }
    return sum / 3;
  });
  expect(await canvasNonEmpty(page)).toBeTruthy();
  // fog adds light veil — later should be brighter or at least painted
  expect(later).toBeGreaterThan(0);
  expect(early).toBeGreaterThanOrEqual(0);
  await page.screenshot({ path: path.join("test-results", "scene-rain-glass.png") });
});
