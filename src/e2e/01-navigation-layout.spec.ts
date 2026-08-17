import { test, expect } from "@playwright/test";
import { setupTauriMocks, assertNoConsoleErrors } from "./mocks/tauri-mock";

test.describe("Suite 1: Navigation, Sidebar & Layout (15 Scenarios)", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test("1.1 Initial load defaults to AI Assistant tab with header badge", async ({ page }) => {
    await expect(page.locator("header")).toContainText("LOCUS");
    await expect(page.locator("header")).toContainText("LOCAL-FIRST IDE");
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("AI Assistant");
    assertNoConsoleErrors(page);
  });

  test("1.2 Navigate to Workspace & Diffs via sidebar click", async ({ page }) => {
    await page.click("nav button:has-text('Workspace & Diffs')");
    await expect(page.locator("body")).toContainText(/Indexed Files|Staged Changes/i);
    assertNoConsoleErrors(page);
  });

  test("1.3 Navigate to P2P Mesh Network via sidebar click", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    await expect(page.locator("h2")).toContainText("P2P Mesh Network");
    assertNoConsoleErrors(page);
  });

  test("1.4 Navigate to Settings & Cloud via sidebar click", async ({ page }) => {
    await page.click("nav button:has-text('Settings & Cloud')");
    await expect(page.locator("body")).toContainText(/Cloud AI Providers|Keyring Vault/i);
    assertNoConsoleErrors(page);
  });

  test("1.5 Navigate to Diagnostics Audit via sidebar click", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("h2")).toContainText("Diagnostics & Support Audit Exporter");
    assertNoConsoleErrors(page);
  });

  test("1.6 Keyboard shortcut Ctrl+1 switches to Chat tab", async ({ page }) => {
    await page.click("nav button:has-text('Settings & Cloud')");
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "1", ctrlKey: true, bubbles: true })));
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("AI Assistant");
    assertNoConsoleErrors(page);
  });

  test("1.7 Keyboard shortcut Ctrl+2 switches to Workspace tab", async ({ page }) => {
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "2", ctrlKey: true, bubbles: true })));
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("Workspace & Diffs");
    assertNoConsoleErrors(page);
  });

  test("1.8 Keyboard shortcut Ctrl+3 switches to Network tab", async ({ page }) => {
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "3", ctrlKey: true, bubbles: true })));
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("P2P Mesh Network");
    assertNoConsoleErrors(page);
  });

  test("1.9 Keyboard shortcut Ctrl+4 switches to Settings tab", async ({ page }) => {
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "4", ctrlKey: true, bubbles: true })));
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("Settings & Cloud");
    assertNoConsoleErrors(page);
  });

  test("1.10 Keyboard shortcut Ctrl+5 switches to Diagnostics tab", async ({ page }) => {
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "5", ctrlKey: true, bubbles: true })));
    const activeTab = page.locator("nav button.bg-gradient-to-r");
    await expect(activeTab).toContainText("Diagnostics Audit");
    assertNoConsoleErrors(page);
  });

  test("1.11 Keyboard shortcut Ctrl+B collapses sidebar to compact icon mode", async ({ page }) => {
    const aside = page.locator("aside");
    await expect(aside).toHaveClass(/w-60/);
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "b", ctrlKey: true, bubbles: true })));
    await expect(aside).toHaveClass(/w-16/);
    assertNoConsoleErrors(page);
  });

  test("1.12 Keyboard shortcut Ctrl+B re-expands sidebar to full width", async ({ page }) => {
    const aside = page.locator("aside");
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "b", ctrlKey: true, bubbles: true })));
    await expect(aside).toHaveClass(/w-16/);
    await page.evaluate(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "b", ctrlKey: true, bubbles: true })));
    await expect(aside).toHaveClass(/w-60/);
    assertNoConsoleErrors(page);
  });

  test("1.13 Top header chevron button toggles sidebar collapse", async ({ page }) => {
    const aside = page.locator("aside");
    const topChevron = page.locator("header button[aria-label*='sidebar']");
    await topChevron.click();
    await expect(aside).toHaveClass(/w-16/);
    await topChevron.click();
    await expect(aside).toHaveClass(/w-60/);
    assertNoConsoleErrors(page);
  });

  test("1.14 Bottom sidebar collapse button toggles sidebar state", async ({ page }) => {
    const aside = page.locator("aside");
    const bottomToggle = page.locator("aside button:has-text('Collapse')");
    await bottomToggle.click();
    await expect(aside).toHaveClass(/w-16/);
    assertNoConsoleErrors(page);
  });

  test("1.15 Sidebar tooltips and hover attributes exist when collapsed", async ({ page }) => {
    await page.keyboard.press("Control+b");
    const chatButton = page.locator("aside nav button").first();
    const titleAttr = await chatButton.getAttribute("title");
    expect(titleAttr).toContain("AI Assistant");
    assertNoConsoleErrors(page);
  });
});
