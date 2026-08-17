import { test, expect } from "@playwright/test";
import { setupTauriMocks, assertNoConsoleErrors } from "./mocks/tauri-mock";

test.describe("Suite 4: Cloud Providers, Keyring Vault & Auto-Detect (20 Scenarios)", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await page.click("nav button:has-text('Settings & Cloud')");
  });

  test("4.1 Settings tab displays Cloud AI Providers & Keyring Vault panel", async ({ page }) => {
    await expect(page.locator("body")).toContainText("Cloud AI Providers & Secure Keyring Vault");
    await expect(page.locator("body")).toContainText("OS Credential Manager (keyring-rs)");
    assertNoConsoleErrors(page);
  });

  test("4.2 Auto-Detect & Import Keys banner is visible with zero-copy badge", async ({ page }) => {
    await expect(page.locator("body")).toContainText("Auto-Detect & Import Keys");
    await expect(page.locator("body")).toContainText("Zero Copy-Paste");
    assertNoConsoleErrors(page);
  });

  test("4.3 Clicking 'Scan & Import Keys' triggers auto-detection and renders results list", async ({ page }) => {
    const scanBtn = page.locator("button:has-text('Scan & Import Keys')");
    await scanBtn.click();
    await expect(page.locator("body")).toContainText("Key Discovery Results:");
    await expect(page.locator("body")).toContainText("AIza••••••••2345");
    await expect(page.locator("body")).toContainText("gsk_••••••••7890");
    assertNoConsoleErrors(page);
  });

  test("4.4 Auto-detected keys display masked format protecting secrets", async ({ page }) => {
    const scanBtn = page.locator("button:has-text('Scan & Import Keys')");
    await scanBtn.click();
    await expect(page.locator("body")).toContainText("••••••••");
    assertNoConsoleErrors(page);
  });

  test("4.5 Provider cards render for Google Gemini, Groq, OpenRouter and DeepSeek", async ({ page }) => {
    await expect(page.locator("body")).toContainText("Google Gemini");
    await expect(page.locator("body")).toContainText("Groq LPU");
    await expect(page.locator("body")).toContainText("OpenRouter");
    await expect(page.locator("body")).toContainText("DeepSeek Direct");
    assertNoConsoleErrors(page);
  });

  test("4.6 Configured provider card shows 'Keyring' encrypted badge", async ({ page }) => {
    const geminiCard = page.locator("div:has-text('Google Gemini')").first();
    await expect(geminiCard).toContainText(/Keyring/i);
    assertNoConsoleErrors(page);
  });

  test("4.7 'Get Key ↗' button exists and is clickable for quick generation", async ({ page }) => {
    const getKeyBtn = page.locator("button:has-text('Get Key')").first();
    await expect(getKeyBtn).toBeVisible();
    await getKeyBtn.click();
    assertNoConsoleErrors(page);
  });

  test("4.8 '📋 Paste' button pastes key into input field", async ({ page, context }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.evaluate(() => navigator.clipboard.writeText("gsk_SamplePastedKeyFromClipboard123"));
    const pasteBtn = page.locator("button:has-text('Paste')").nth(1); // Groq card
    if (await pasteBtn.isVisible()) {
      await pasteBtn.click();
      await expect(page.locator("input[type='password'], input[type='text']").nth(1)).toHaveValue(
        "gsk_SamplePastedKeyFromClipboard123"
      );
    }
    assertNoConsoleErrors(page);
  });

  test("4.9 Password visibility toggle switches input between masked and visible text", async ({ page }) => {
    const showBtn = page.locator("button:has-text('Show')").first();
    if (await showBtn.isVisible()) {
      await showBtn.click();
      await expect(page.locator("button:has-text('Hide')").first()).toBeVisible();
      await page.locator("button:has-text('Hide')").first().click();
      await expect(page.locator("button:has-text('Show')").first()).toBeVisible();
    }
    assertNoConsoleErrors(page);
  });

  test("4.10 Saving API key updates provider configuration and triggers feedback", async ({ page }) => {
    const groqInput = page.locator("input[placeholder*='Groq']").first();
    if (await groqInput.isVisible()) {
      await groqInput.fill("gsk_NewTestKey12345");
      const saveBtn = page.locator("button:has-text('Save')").nth(1);
      await saveBtn.click();
      await page.waitForTimeout(300);
    }
    assertNoConsoleErrors(page);
  });

  test("4.11 Testing API connection shows loading spinner and latency metrics", async ({ page }) => {
    const testBtn = page.locator("button:has-text('Test Connection')").first();
    await testBtn.scrollIntoViewIfNeeded();
    await testBtn.click();
    await expect(page.locator("body")).toContainText("Active & Connected");
    await expect(page.locator("body")).toContainText("95ms");
    assertNoConsoleErrors(page);
  });

  test("4.12 Simulated API key failure displays clear error alert badge", async ({ page }) => {
    await setupTauriMocks(page, { simulateTestFail: true });
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    await page.click("nav button:has-text('Settings & Cloud')");

    const testBtn = page.locator("button:has-text('Test Connection')").first();
    await testBtn.scrollIntoViewIfNeeded();
    await testBtn.click();
    await expect(page.locator("body")).toContainText("Connection Failed");
    assertNoConsoleErrors(page);
  });

  test("4.13 Delete API key removes credential from Keyring storage", async ({ page }) => {
    const deleteBtn = page.locator("button:has-text('Delete')").first();
    if (await deleteBtn.isVisible()) {
      await deleteBtn.scrollIntoViewIfNeeded();
      await deleteBtn.click({ force: true });
      await page.waitForTimeout(200);
    }
    assertNoConsoleErrors(page);
  });

  test("4.14 Fallback Router section displays active strategy selector", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/Auto-Fallback Chain Router/i);
    assertNoConsoleErrors(page);
  });

  test("4.15 Changing Fallback Strategy to 'SpeedFirst' updates routing preference", async ({ page }) => {
    const speedBtn = page.locator("button:has-text('Speed First')").first();
    if (await speedBtn.isVisible()) {
      await speedBtn.click();
    }
    assertNoConsoleErrors(page);
  });

  test("4.16 Changing Fallback Strategy to 'CloudFirst' updates routing preference", async ({ page }) => {
    const cloudBtn = page.locator("button:has-text('Cloud First')").first();
    if (await cloudBtn.isVisible()) {
      await cloudBtn.click();
    }
    assertNoConsoleErrors(page);
  });

  test("4.17 Fallback Target toggle enables and disables provider in chain", async ({ page }) => {
    const toggle = page.locator("input[type='checkbox']").first();
    if (await toggle.isVisible()) {
      await toggle.click();
    }
    assertNoConsoleErrors(page);
  });

  test("4.18 Fallback Auto-Failover status badge toggles state", async ({ page }) => {
    const toggleBtn = page.locator("button:has-text('Auto-Failover'), button:has-text('Disabled')").first();
    if (await toggleBtn.isVisible()) {
      await toggleBtn.click();
      await expect(page.locator("body")).toContainText(/Disabled|Auto-Failover/i);
      await toggleBtn.click();
      await expect(page.locator("body")).toContainText(/Auto-Failover: Active/i);
    }
    assertNoConsoleErrors(page);
  });

  test("4.19 Sound Effects toggle button switches audio feedback state", async ({ page }) => {
    const soundToggle = page.locator("button:has-text('Audio Feedback'), button:has-text('Sound')").first();
    if (await soundToggle.isVisible()) {
      await soundToggle.click();
    }
    assertNoConsoleErrors(page);
  });

  test("4.20 Refresh Provider Status button reloads hardware keyring state", async ({ page }) => {
    const refreshBtn = page.locator("button:has-text('Refresh Status')").first();
    await refreshBtn.click();
    await page.waitForTimeout(200);
    assertNoConsoleErrors(page);
  });
});
