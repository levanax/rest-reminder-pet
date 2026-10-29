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

test("玻璃起雾场景有雾层像素", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/e2e/fixtures/scene-mock.html?scene=rainGlass");
  await page.waitForTimeout(400);
  const early = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const d = ctx.getImageData(c.width / 2, c.height / 2, 1, 1).data;
    return d[0] + d[1] + d[2];
  });
  await page.waitForTimeout(2500);
  const later = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
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
  expect(later).toBeGreaterThan(0);
  expect(early).toBeGreaterThanOrEqual(0);
  await page.screenshot({ path: path.join("test-results", "scene-rain-glass.png") });
});

test("玻璃下雨场景有雨痕像素", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/e2e/fixtures/scene-mock.html?scene=rainStreak");
  await page.waitForTimeout(1200);
  expect(await canvasNonEmpty(page)).toBeTruthy();
  await page.screenshot({ path: path.join("test-results", "scene-rain-streak.png") });
});

test("百叶窗场景先放下再合上", async ({ page }) => {
  await page.setViewportSize({ width: 800, height: 600 });
  await page.goto("/e2e/fixtures/scene-mock.html?scene=blinds");

  // early: rail / open slats near top, bottom still mostly clear
  await page.waitForTimeout(500);
  const early = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const top = ctx.getImageData(Math.floor(c.width * 0.5), Math.floor(c.height * 0.08), 1, 1).data;
    const bottom = ctx.getImageData(Math.floor(c.width * 0.5), Math.floor(c.height * 0.92), 1, 1).data;
    return { topAlpha: top[3], bottomAlpha: bottom[3] };
  });
  expect(early.topAlpha).toBeGreaterThan(8);
  expect(early.bottomAlpha).toBeLessThan(40);

  // after drop + hold: stack covers mid-screen but open gaps remain (some clear pixels)
  await page.waitForTimeout(2800);
  await page.screenshot({ path: path.join("test-results", "scene-blinds-open.png") });
  const mid = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const x = Math.floor(c.width * 0.5);
    const y0 = Math.floor(c.height * 0.35);
    const y1 = Math.floor(c.height * 0.65);
    const strip = ctx.getImageData(x, y0, 1, y1 - y0).data;
    let opaque = 0;
    let clear = 0;
    for (let i = 3; i < strip.length; i += 4) {
      if (strip[i] > 20) opaque++;
      else clear++;
    }
    return { opaque, clear, total: opaque + clear };
  });
  expect(mid.opaque).toBeGreaterThan(10);
  expect(mid.clear).toBeGreaterThan(10);

  // deep into slow close: gaps mostly gone
  await page.waitForTimeout(7000);
  expect(await canvasNonEmpty(page)).toBeTruthy();
  const late = await page.evaluate(() => {
    const c = document.getElementById("snow") as HTMLCanvasElement;
    const ctx = c.getContext("2d")!;
    const x = Math.floor(c.width * 0.5);
    const y0 = Math.floor(c.height * 0.35);
    const y1 = Math.floor(c.height * 0.65);
    const strip = ctx.getImageData(x, y0, 1, y1 - y0).data;
    let opaque = 0;
    for (let i = 3; i < strip.length; i += 4) {
      if (strip[i] > 20) opaque++;
    }
    return { opaque, total: strip.length / 4 };
  });
  expect(late.opaque / late.total).toBeGreaterThan(0.7);
  await page.screenshot({ path: path.join("test-results", "scene-blinds-closing.png") });
});
