import { test, expect } from "@playwright/test";

test("published releases provide separate Windows, Linux and both Mac downloads", async ({
  page,
}) => {
  const assets = [
    ["Linux", "openbox-x86_64-unknown-linux-gnu.tar.gz"],
    ["Windows", "openbox-x86_64-pc-windows-msvc.zip"],
    ["Apple silicon", "openbox-aarch64-apple-darwin.tar.gz"],
    ["Intel Mac", "openbox-x86_64-apple-darwin.tar.gz"],
  ];
  await page.route("https://api.github.com/**", (route) =>
    route.fulfill({
      json: [
        {
          tag_name: "v0.2.0",
          draft: false,
          prerelease: false,
          assets: assets.map(([, name]) => ({ name })),
        },
      ],
    }),
  );
  await page.goto("/");
  for (const [platform, asset] of assets) {
    await expect(
      page.getByRole("link", {
        name: `Download ${platform} v0.2.0`,
        exact: true,
      }),
    ).toHaveAttribute(
      "href",
      `https://github.com/alexanderwanyoike/openbox/releases/download/v0.2.0/${asset}`,
    );
  }
  await expect(
    page
      .getByRole("region", { name: "Features" })
      .getByText(/Device Library and OneLibrary/),
  ).toBeVisible();
});

test("the project site shows the actual workspace and valid release navigation", async ({
  page,
}) => {
  await page.route("https://api.github.com/**", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await expect(
    page.getByAltText("Openbox library, decks and mixer"),
  ).toHaveJSProperty(
    "currentSrc",
    "http://127.0.0.1:4173/assets/workspace-light.png",
  );
  await expect(
    page.getByAltText(
      "Local collection beside a USB playlist with shared editing controls",
    ),
  ).toHaveJSProperty(
    "currentSrc",
    "http://127.0.0.1:4173/assets/usb-playlist-light.png",
  );
  await expect(page.locator("html")).toHaveCSS(
    "background-color",
    "rgb(242, 242, 242)",
  );
  await page.emulateMedia({ colorScheme: "dark" });
  await expect(page).toHaveTitle("Openbox");
  await expect(page.locator("html")).toHaveCSS(
    "background-color",
    "rgb(16, 16, 16)",
  );
  await expect(
    page.getByRole("heading", { name: "Openbox", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByAltText("Openbox library, decks and mixer"),
  ).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Linux releases", exact: true }),
  ).toHaveAttribute(
    "href",
    "https://github.com/alexanderwanyoike/openbox/releases",
  );
  await page.screenshot({ path: "site/artifacts/desktop.png", fullPage: true });
});

test("published Linux releases link to their versioned binary", async ({
  page,
}) => {
  await page.route("https://api.github.com/**", (route) =>
    route.fulfill({
      json: [
        {
          tag_name: "v0.2.0",
          draft: false,
          prerelease: false,
          assets: [
            {
              name: "openbox-x86_64-unknown-linux-gnu.tar.gz",
              browser_download_url:
                "https://github.com/alexanderwanyoike/openbox/releases/download/v0.2.0/openbox-x86_64-unknown-linux-gnu.tar.gz",
            },
          ],
        },
      ],
    }),
  );
  await page.goto("/");
  await expect(
    page.getByRole("link", { name: "Download Linux v0.2.0" }),
  ).toHaveAttribute(
    "href",
    /\/v0.2.0\/openbox-x86_64-unknown-linux-gnu.tar.gz$/,
  );
});

test("mobile layout fits without horizontal overflow and retains offline release navigation", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.route("https://api.github.com/**", (route) => route.abort());
  await page.goto("/");
  await expect(
    page.getByRole("link", { name: "Linux releases", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "site/artifacts/mobile.png", fullPage: true });
});

test("product copy describes USB management without naming another application", async ({
  page,
}) => {
  await page.route("https://api.github.com/**", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/");
  await expect(page.locator("body")).not.toContainText(/rekordbox/i);
  await expect(page.locator('meta[name="description"]')).not.toHaveAttribute(
    "content",
    /rekordbox/i,
  );
  await expect(
    page.getByRole("heading", { name: "USB libraries", exact: true }),
  ).toBeVisible();
});
