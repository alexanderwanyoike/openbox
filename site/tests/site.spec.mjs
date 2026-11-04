import { test, expect } from "@playwright/test";

test("the project site shows the actual workspace and valid release navigation", async ({
  page,
}) => {
  await page.route("https://api.github.com/**", (route) =>
    route.fulfill({ json: [] }),
  );
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/");
  await expect(page.locator("html")).toHaveCSS("background-color", "rgb(242, 242, 242)");
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
    page.getByRole("link", { name: "View releases" }),
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
    page.getByRole("link", { name: "Download v0.2.0" }),
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
  await expect(page.getByRole("link", { name: "View releases" })).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: "site/artifacts/mobile.png", fullPage: true });
});
