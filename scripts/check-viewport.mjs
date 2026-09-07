/**
 * Viewport guard: drives the browser demo through every page and asserts that
 * nothing scrolls at the Tauri minimum (960x640), default (1180x760) and a
 * large window. Screenshots land in .ui-shots/ for review.
 *
 * Requires Google Chrome (playwright-core `channel: "chrome"`) and a Vite dev
 * server; the script starts one on port 1431 unless VIEWPORT_BASE is set.
 */
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { chromium, webkit } from "playwright-core";

const sizes = [
  { name: "min", width: 960, height: 640 },
  { name: "default", width: 1180, height: 760 },
  { name: "large", width: 1440, height: 900 },
];
const selected = process.env.VIEWPORT_SIZES?.split(",");
const base = process.env.VIEWPORT_BASE ?? "http://localhost:1431";
let server;
if (!process.env.VIEWPORT_BASE) {
  server = spawn("pnpm", ["exec", "vite", "--port", "1431", "--strictPort"], {
    stdio: "ignore",
  });
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      await fetch(base);
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}
mkdirSync(".ui-shots", { recursive: true });

const failures = [];
// WKWebView lays out like Playwright's WebKit; VIEWPORT_BROWSER=webkit checks it.
const browser =
  process.env.VIEWPORT_BROWSER === "webkit"
    ? await webkit.launch({ headless: true })
    : await chromium.launch({ channel: "chrome", headless: true });
try {
  for (const size of sizes.filter(
    (item) => !selected || selected.includes(item.name),
  )) {
    const context = await browser.newContext({
      viewport: { width: size.width, height: size.height - 22 },
      locale: "zh-CN",
    });
    const page = await context.newPage();
    const check = async (name) => {
      await page.waitForTimeout(400);
      const result = await page.evaluate(() => {
        const scrollers = [
          ...document.querySelectorAll("main *, dialog[open] *"),
        ]
          .filter((element) => {
            // Consent text is the one region allowed to scroll; it opts out.
            if (element.hasAttribute("data-scroll-ok")) return false;
            const style = getComputedStyle(element);
            return (
              /(auto|scroll)/.test(style.overflowY + style.overflowX) &&
              (element.scrollHeight > element.clientHeight + 1 ||
                element.scrollWidth > element.clientWidth + 1)
            );
          })
          .map((element) => element.tagName.toLowerCase());
        // Hidden overflow that actually cuts content is as bad as a scrollbar;
        // single-line truncation and clamped text opt out.
        const clipped = [
          ...document.querySelectorAll("main, main *, dialog[open] *"),
        ]
          .filter((element) => {
            const style = getComputedStyle(element);
            const classes = element.className?.toString() ?? "";
            return (
              style.overflowY === "hidden" &&
              !/truncate|line-clamp|rounded-full|will-change-transform|\bclip\b/.test(
                classes,
              ) &&
              element.scrollHeight > element.clientHeight + 2
            );
          })
          .map(
            (element) =>
              `${element.tagName.toLowerCase()}.${[...element.classList].slice(0, 3).join(".")}`,
          );
        return { scrollers, clipped };
      });
      await page.screenshot({ path: `.ui-shots/${size.name}-${name}.png` });
      const problems = [...result.clipped, ...result.scrollers];
      if (problems.length)
        failures.push(`${size.name}/${name}: ${problems.join(", ")}`);
      console.log(
        `${problems.length ? "✗" : "✓"} ${size.name} ${name}${problems.length ? ` → ${problems.join(", ")}` : ""}`,
      );
    };
    const clickText = async (pattern) => {
      const button = page.getByRole("button", { name: pattern }).first();
      await button.waitFor({ timeout: 15000 });
      await button.click();
    };
    const side = (index) => page.locator("aside nav button").nth(index).click();
    const nav = (index) => page.locator("header nav button").nth(index).click();
    // Installed apps and logs are sidebar entries under the device tab.
    const sideItem = (text) =>
      page.locator("aside button", { hasText: text }).first().click();

    await page.goto(base);
    await page.waitForSelector("aside", { timeout: 20000 });
    await page.waitForTimeout(1200);
    await check("device-summary");
    await side(1);
    await check("device-conditions");
    await side(2);
    await check("device-environment");
    await clickText(/开始前置准备/);
    await check("preparation-overview");
    for (const box of await page.locator("main input[type=checkbox]").all())
      if (!(await box.isChecked())) await box.check();
    await clickText(/我已确认，查看风险/);
    await check("consent-risks");
    for (let turn = 0; turn < 6; turn += 1) {
      const next = page.locator("dialog[open] button", { hasText: "下一页" });
      if (!(await next.count()) || (await next.isDisabled())) break;
      await next.click();
      await page.waitForTimeout(200);
    }
    await page.waitForTimeout(5200);
    await clickText(/我已理解并确认全部风险/);
    await check("consent-disclaimer");
    for (let turn = 0; turn < 6; turn += 1) {
      const next = page.locator("dialog[open] button", { hasText: "下一页" });
      if (!(await next.count()) || (await next.isDisabled())) break;
      await next.click();
      await page.waitForTimeout(200);
    }
    await page.waitForTimeout(5200);
    await clickText(/同意并开始/);
    await page.waitForTimeout(1000);
    await check("dfu-idle");
    await clickText(/已完全关机，开始引导/);
    await page.waitForTimeout(2500);
    await check("dfu-hold");
    await page.waitForTimeout(18500);
    await check("execution");
    await page
      .getByRole("button", { name: /前往应用商店|重试|重新检测设备|关闭/ })
      .first()
      .waitFor({ timeout: 120000 });
    await check("result");
    await nav(1);
    await check("store");
    await page.locator("main article button").first().click();
    await check("package-detail");
    await nav(0);
    await sideItem("已安装应用");
    await check("installed");
    await sideItem("操作日志");
    await check("logs");
    await page.getByRole("button", { name: "偏好设置" }).last().click();
    await check("settings");
    await page.keyboard.press("Escape");
    await context.close();
  }
} finally {
  await browser.close();
  server?.kill();
}
if (failures.length) {
  console.error(`\nOverflow detected:\n${failures.join("\n")}`);
  process.exit(1);
}
console.log("\nEvery page fits the viewport.");
