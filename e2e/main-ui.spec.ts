import { expect, test } from "@playwright/test";
import path from "node:path";

test("今日页 KPI 与棒棒糖图可见", async ({ page }) => {
  await page.goto("/e2e/fixtures/today-mock.html");
  await expect(page.getByRole("heading", { name: "Rest Reminder Pet" })).toBeVisible();
  await expect(page.locator(".kpi .v").first()).toHaveText("3");
  await expect(page.locator("svg.lollipop")).toBeVisible();
  await expect(page.locator(".event-list li")).toHaveCount(4);
  await page.screenshot({
    path: path.join("test-results", "today-panel.png"),
    fullPage: true,
  });
});
