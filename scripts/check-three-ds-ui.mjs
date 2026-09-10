import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import { chromium } from "playwright-core";
import { createServer } from "vite";
const server = await createServer({ server: { host: "127.0.0.1", port: 0 } });
let browser;
try {
  await server.listen();
  const address = server.httpServer.address();
  assert.ok(address && typeof address === "object");
  const base = `http://127.0.0.1:${address.port}`;
  browser = await chromium.launch({ channel: "chrome", headless: true });
  mkdirSync(".ui-shots", { recursive: true });
  for (const locale of ["zh-CN", "en"]) {
    for (const theme of ["light", "dark"]) {
      const context = await browser.newContext({
        viewport: { width: 960, height: 618 },
        locale,
      });
      await context.addInitScript(
        ({ locale, theme }) => {
          localStorage.setItem("pocket-studio.locale", locale);
          localStorage.setItem("pocket-studio.theme", theme);
          localStorage.setItem(
            "pocket-studio.three-ds-connection",
            JSON.stringify({
              address: "192.0.2.1",
              port: "5000",
              username: "anonymous",
              password: "",
            }),
          );
        },
        { locale, theme },
      );
      const page = await context.newPage();
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      const click = async (name) => {
        const root = (await page.locator("dialog[open]").count())
          ? page.locator("dialog[open]")
          : page;
        const button = root
          .getByRole("button", { name, exact: true })
          .filter({ visible: true })
          .first();
        try {
          await button.click({ timeout: 5000 });
        } catch (error) {
          console.error(
            await button.evaluate((el) => {
              const r = el.getBoundingClientRect();
              return {
                html: el.outerHTML,
                rect: r.toJSON(),
                hit: document
                  .elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)
                  ?.outerHTML.slice(0, 800),
                dialogs: [...document.querySelectorAll("dialog[open]")].map(
                  (d) => ({
                    rect: d.getBoundingClientRect().toJSON(),
                    html: d.outerHTML.slice(0, 800),
                  }),
                ),
              };
            }),
          );
          throw error;
        }
      };
      const shot = async (name) => {
        await page.waitForTimeout(250);
        const overflow = await page.evaluate(
          () =>
            document.documentElement.scrollWidth > innerWidth ||
            document.documentElement.scrollHeight > innerHeight,
        );
        if (overflow) throw new Error(`3DS viewport overflow: ${name}`);
        await page.screenshot({
          path: `.ui-shots/3ds-${locale}-${theme}-${name}.png`,
        });
      };
      await page.goto(base);
      await page.waitForSelector("aside");
      await page.waitForTimeout(500);
      const review = locale === "en" ? "Review preparation" : "查看准备方案";
      const edit = locale === "en" ? "Edit plan" : "修改方案";
      const write = locale === "en" ? "Write listed files" : "写入所列文件";
      const verify = locale === "en" ? "Verify connection" : "验证连接";
      const wireless = locale === "en" ? "Wireless (ftpd)" : "无线（ftpd）";
      const reader = locale === "en" ? "SD card reader" : "SD 卡读卡器";
      const cardPath = locale === "en" ? "SD card root folder" : "SD 卡根目录";
      const dialog = page.locator("dialog[open]");
      // Without a launcher (or a console) a standalone title still reaches the
      // card: the copy flow lists the CIA before anything is paired.
      await page.locator("header nav button").nth(1).click();
      await page.waitForTimeout(500);
      await page.getByRole("radio", { name: "3DS", exact: true }).click();
      await page.getByRole("searchbox").fill("Theme Forge");
      await page.waitForTimeout(300);
      await page
        .locator("main article", { hasText: "Theme Forge" })
        .getByRole("heading")
        .click();
      await page.waitForTimeout(300);
      await click(locale === "en" ? "Copy to SD card…" : "复制到 SD 卡…");
      assert.ok(
        (await dialog.innerText()).includes(
          locale === "en" ? "Theme Forge" : "Theme Forge",
        ),
      );
      await shot("card");
      await click(review);
      assert.ok(
        (await dialog.innerText()).includes("cias/ctr-theme-forge-"),
        "standalone CIA is staged for FBI",
      );
      await shot("card-plan");
      await click(locale === "en" ? "Close" : "关闭");
      await page.getByRole("searchbox").fill("");
      await page.locator("header nav button").nth(0).click();
      await page.waitForTimeout(300);
      // Manual connection is device-generic; the 3DS is the first kind.
      await page.locator("header details summary").first().click();
      await click(
        `${locale === "en" ? "Connect a device manually" : "手动连接设备"}…`,
      );
      await shot("connect-kinds");
      await dialog
        .getByRole("button", { name: /Nintendo 3DS/ })
        .first()
        .click();
      // Preferences hold the console address, so wireless is preselected and
      // the dialog only summarises the ftpd account.
      assert.equal(
        await dialog
          .getByRole("radio", { name: wireless, exact: true })
          .getAttribute("aria-checked"),
        "true",
      );
      assert.ok((await dialog.innerText()).includes("ftpd 192.0.2.1:5000"));
      assert.equal(
        await dialog.getByLabel(cardPath, { exact: true }).count(),
        0,
      );
      assert.equal(
        await dialog.getByRole("combobox").count(),
        0,
        "no launcher choice in the connect flow",
      );
      await shot("connect-wireless");
      await page.evaluate(async () => {
        const { useGateway, GatewayError } =
          await import("/src/shared/gateway/index.ts");
        const gateway = useGateway();
        const original = gateway.setup.plan;
        gateway.setup.plan = async () => {
          gateway.setup.plan = original;
          throw new GatewayError("ftpRootUnavailable", "Remote root fixture");
        };
      });
      await click(review);
      const remoteError = dialog.getByRole("alert");
      await remoteError
        .getByText("ftpRootUnavailable", { exact: false })
        .waitFor();
      const remoteMessage = await remoteError.innerText();
      assert.ok(remoteMessage.includes("ftpd"));
      assert.ok(
        !remoteMessage.includes(
          locale === "en" ? "Select a prepared" : "请选择",
        ),
      );
      await shot("connect-ftp-root-unavailable");
      await click(review);
      assert.ok((await dialog.innerText()).includes("ftpd 192.0.2.1:5000"));
      // Pairing only: the key is the single file written.
      assert.deepEqual(await dialog.locator("li").allTextContents(), [
        "pocketjs/runtime/dev.key",
      ]);
      await shot("connect-plan");
      await click(edit);
      await dialog.getByRole("radio", { name: reader, exact: true }).click();
      await dialog.getByLabel(cardPath, { exact: true }).fill("/demo/3ds-card");
      await shot("connect-sd");
      await click(review);
      await click(write);
      // The console-side checklist ends with the Runtime menu before verifying.
      assert.ok((await dialog.innerText()).includes("L+R+SELECT"));
      await shot("connect-written");
      await click(verify);
      if (
        (await page.locator("header details").first().getAttribute("open")) ===
        null
      )
        await page.locator("header details summary").first().click();
      await click("New Nintendo 3DS LL");
      await page.locator("header details summary").first().click();
      await shot("device");
      // The launcher is prepared from the environment page, not while connecting.
      await page
        .locator("aside button", {
          hasText: locale === "en" ? "Preparation" : "前置环境",
        })
        .first()
        .click();
      await click(
        locale === "en"
          ? "Install or upgrade the launcher"
          : "安装或升级启动器",
      );
      const launcherForm = dialog.getByRole("radiogroup", {
        name: locale === "en" ? "Launcher form" : "启动器形式",
      });
      assert.equal(
        await launcherForm
          .getByRole("radio", { name: /CIA/ })
          .getAttribute("aria-checked"),
        "true",
      );
      await shot("launcher");
      await launcherForm.getByRole("radio", { name: /3DSX/ }).click();
      // Store failures surface as such, without blaming the destination.
      await page.evaluate(async () => {
        const { useGateway, GatewayError } =
          await import("/src/shared/gateway/index.ts");
        const gateway = useGateway();
        const original = gateway.setup.plan;
        const failures = ["invalidCatalog", "storeOffline"];
        gateway.setup.plan = async (...args) => {
          const code = failures.shift();
          if (code) throw new GatewayError(code, "Store failure fixture");
          gateway.setup.plan = original;
          return original(...args);
        };
      });
      for (const code of ["invalidCatalog", "storeOffline"]) {
        await click(review);
        const alert = dialog.getByRole("alert");
        await alert.getByText(code, { exact: false }).waitFor();
        const message = await alert.innerText();
        assert.ok(
          locale === "en" ? /store/i.test(message) : message.includes("商店"),
        );
        assert.ok(
          !message.includes(
            locale === "en" ? "Check the destination" : "请检查目标",
          ),
        );
        assert.equal(
          await dialog
            .getByRole("button", { name: write, exact: true })
            .count(),
          0,
        );
        await shot(`launcher-${code}`);
      }
      await click(review);
      assert.ok(
        (await dialog.innerText()).includes("3ds/pocket-runtime-ctr/boot.3dsx"),
      );
      await shot("launcher-plan");
      await click(write);
      assert.ok(
        (await dialog.innerText()).includes("Homebrew Launcher"),
        "a 3DSX launcher is started from the Homebrew Launcher",
      );
      await shot("launcher-written");
      await click(locale === "en" ? "Close" : "关闭");
      const install = locale === "en" ? "Install" : "安装";
      const uninstall = locale === "en" ? "Uninstall" : "卸载";
      const confirmUninstall =
        locale === "en" ? "Confirm uninstall" : "确认卸载";
      const removeWithData =
        locale === "en" ? "Remove and clear data" : "卸载并清除数据";
      const standalone = locale === "en" ? "Standalone app" : "独立应用";
      const search = page.getByRole("searchbox");
      const dialogs = page.locator("dialog[open]");
      const card = (name) => page.locator("main article", { hasText: name });
      // The storefront follows the console: the 3DS shelves are shown.
      await page.locator("header nav button").nth(1).click();
      await page.waitForTimeout(500);
      await shot("storefront");
      await search.fill("Dual Notebook");
      await page.waitForTimeout(300);
      // Several published forms: the delivery choice is the one dialog.
      await card("Dual Notebook")
        .getByRole("button", { name: install, exact: true })
        .click();
      await dialogs.getByRole("radio", { name: standalone }).waitFor();
      await shot("delivery");
      await click(install);
      await page.waitForTimeout(300);
      assert.equal(await dialogs.count(), 0, "install runs without review");
      // The detail page still offers the other form after one is installed.
      await card("Dual Notebook").getByRole("heading").click();
      await page.waitForTimeout(300);
      await page
        .locator("main")
        .getByRole("button", { name: install, exact: true })
        .first()
        .click();
      await dialogs.getByRole("radio", { name: standalone }).click();
      await shot("delivery-standalone");
      await click(install);
      await page.waitForTimeout(300);
      // A title with a single published form installs straight away.
      await page.locator("header nav button").nth(1).click();
      await search.fill("Theme Forge");
      await page.waitForTimeout(300);
      await card("Theme Forge")
        .getByRole("button", { name: install, exact: true })
        .click();
      await page.waitForTimeout(300);
      assert.equal(await dialogs.count(), 0, "single form needs no dialog");
      await page.locator("header nav button").nth(0).click();
      await page
        .locator("aside button", {
          hasText: locale === "en" ? "Installed" : "已安装应用",
        })
        .first()
        .click();
      await page.waitForTimeout(500);
      const rows = page.locator("tbody tr");
      const notebook = rows.filter({ hasText: "Dual Notebook" });
      assert.equal(await rows.count(), 3, "three separate installations");
      assert.equal(await notebook.count(), 2, "two Dual Notebook instances");
      await shot("coexist");
      // Removal confirms inline and keeps app data by default.
      await notebook
        .first()
        .getByRole("button", { name: uninstall, exact: true })
        .click();
      await shot("remove-inline");
      await notebook
        .first()
        .getByRole("button", { name: confirmUninstall, exact: true })
        .click();
      await page.waitForTimeout(300);
      assert.equal(await notebook.count(), 1, "the other instance survived");
      // Wiping data is a separate, explicit choice.
      await notebook
        .first()
        .getByRole("button", { name: uninstall, exact: true })
        .click();
      await notebook
        .first()
        .getByRole("button", { name: removeWithData, exact: true })
        .click();
      await page.waitForTimeout(300);
      assert.equal(await rows.count(), 1, "only Theme Forge remains");
      if (errors.length) throw new Error(errors.join("\n"));
      await context.close();
      console.log(
        `3DS connection, launcher, delivery choice, coexistence and removal passed: ${locale} ${theme}`,
      );
    }
  }
} finally {
  await browser?.close();
  await server.close();
}
