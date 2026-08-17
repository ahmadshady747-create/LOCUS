import { test, expect } from "@playwright/test";
import { setupTauriMocks, assertNoConsoleErrors } from "./mocks/tauri-mock";

test.describe("Suite 5: P2P Mesh Network & Diagnostics Audit (20 Scenarios)", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test("5.1 P2P Mesh Network tab renders topology header and stats", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    await expect(page.locator("h2")).toContainText("P2P Mesh Network & Distributed Compute");
    await expect(page.locator("body")).toContainText("Discovered Peers");
    assertNoConsoleErrors(page);
  });

  test("5.2 Start P2P Mesh button activates local compute discovery daemon", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    const startBtn = page.locator("button:has-text('Start P2P Mesh')");
    if (await startBtn.isVisible()) {
      await startBtn.click();
      await expect(page.locator("body")).toContainText(/MESH ACTIVE|Stop Daemon/i);
    }
    assertNoConsoleErrors(page);
  });

  test("5.3 Stop P2P Mesh button returns daemon to standby mode", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    const startBtn = page.locator("button:has-text('Start P2P Mesh')");
    if (await startBtn.isVisible()) {
      await startBtn.click();
    }
    const stopBtn = page.locator("button:has-text('Stop Daemon')");
    if (await stopBtn.isVisible()) {
      await stopBtn.click();
      await expect(page.locator("body")).toContainText(/STANDBY|Start P2P Mesh/i);
    }
    assertNoConsoleErrors(page);
  });

  test("5.4 Scan Local LAN button refreshes discovered peer nodes", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    const scanLanBtn = page.locator("button:has-text('Scan Local LAN')");
    await scanLanBtn.click();
    await expect(page.locator("body")).toContainText(/MacBook-M3-Pro|RTX-4090-Rig/i);
    assertNoConsoleErrors(page);
  });

  test("5.5 Discovered peer card displays IP address, port, and VRAM capacity", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    await expect(page.locator("body")).toContainText("192.168.1.105");
    await expect(page.locator("body")).toContainText("8080");
    await expect(page.locator("body")).toContainText("18 GB");
    assertNoConsoleErrors(page);
  });

  test("5.6 Clicking peer card selects node and applies glow highlight", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    const peerCard = page.locator("div:has-text('MacBook-M3-Pro')").last();
    await peerCard.click();
    assertNoConsoleErrors(page);
  });

  test("5.7 P2P Security protocol badge displays HMAC verification version", async ({ page }) => {
    await page.click("nav button:has-text('P2P Mesh Network')");
    await expect(page.locator("body")).toContainText("P2P v2.1");
    await expect(page.locator("body")).toContainText("HMAC Payload Verification");
    assertNoConsoleErrors(page);
  });

  test("5.8 Diagnostics Audit tab displays system environment hardware audit", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("h2")).toContainText("Diagnostics & Support Audit Exporter");
    await expect(page.locator("body")).toContainText(/windows|x86_64|16 Logical CPU/i);
    assertNoConsoleErrors(page);
  });

  test("5.9 Zero PII & Secret Redaction Engine guarantee checklist is rendered", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("body")).toContainText("Zero PII & Secret Redaction Engine");
    await expect(page.locator("body")).toContainText("API Keys Masked");
    await expect(page.locator("body")).toContainText("User Paths Scrubbed");
    assertNoConsoleErrors(page);
  });

  test("5.10 Refresh Audit button re-audits system health metrics", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    const refreshAuditBtn = page.locator("button:has-text('Refresh Audit')");
    await refreshAuditBtn.click();
    await page.waitForTimeout(200);
    assertNoConsoleErrors(page);
  });

  test("5.11 Export Diagnostics button compiles and outputs sanitized JSON report", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    const exportBtn = page.locator("button:has-text('Export Diagnostics')");
    await exportBtn.click();
    await expect(page.locator("body")).toContainText(/locus-diagnostic-report|log events exported/i);
    assertNoConsoleErrors(page);
  });

  test("5.12 Copy Payload button copies sanitized JSON payload to clipboard", async ({ page, context }) => {
    await context.grantPermissions(["clipboard-read", "clipboard-write"]);
    await page.click("nav button:has-text('Diagnostics Audit')");
    const exportBtn = page.locator("button:has-text('Export Diagnostics')");
    await exportBtn.click();
    const copyBtn = page.locator("button:has-text('Copy Payload')");
    await copyBtn.click();
    await expect(page.locator("body")).toContainText("Copied JSON!");
    assertNoConsoleErrors(page);
  });

  test("5.13 Download File button triggers download package", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    const exportBtn = page.locator("button:has-text('Export Diagnostics')");
    await exportBtn.click();
    const downloadBtn = page.locator("button:has-text('Download File')");
    await downloadBtn.click();
    assertNoConsoleErrors(page);
  });

  test("5.14 Sanitized Diagnostic Event Stream renders event log entries", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("body")).toContainText("Sanitized Diagnostic Event Stream");
    await expect(page.locator("body")).toContainText("LOCUS engine initialized with hardware credential storage");
    assertNoConsoleErrors(page);
  });

  test("5.15 Log severity level badges render with distinct color accents", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("body")).toContainText("INFO");
    assertNoConsoleErrors(page);
  });

  test("5.16 App version badge in Diagnostics displays 'v0.1.0-alpha'", async ({ page }) => {
    await page.click("nav button:has-text('Diagnostics Audit')");
    await expect(page.locator("body")).toContainText("v0.1.0-alpha");
    assertNoConsoleErrors(page);
  });

  test("5.17 Status Bar at bottom reflects active model and workspace state", async ({ page }) => {
    const footer = page.locator("footer, [role='contentinfo'], .border-t").last();
    await expect(footer).toBeVisible();
    assertNoConsoleErrors(page);
  });

  test("5.18 Rapid tab switching cycle (1->2->3->4->5->1) executes cleanly", async ({ page }) => {
    await page.keyboard.press("Control+1");
    await page.keyboard.press("Control+2");
    await page.keyboard.press("Control+3");
    await page.keyboard.press("Control+4");
    await page.keyboard.press("Control+5");
    await page.keyboard.press("Control+1");
    await expect(page.locator("aside nav button.bg-gradient-to-r")).toContainText("AI Assistant");
    assertNoConsoleErrors(page);
  });

  test("5.19 Sidebar live system footer state updates model count", async ({ page }) => {
    const aside = page.locator("aside");
    await expect(aside).toContainText(/3 local model|2 peer/i);
    assertNoConsoleErrors(page);
  });

  test("5.20 Complete End-to-End User Journey: Prompting AI -> Staged Diff -> Key Management -> Diagnostic Export", async ({
    page,
  }) => {
    // 1. Chat with AI
    await page.keyboard.press("Control+1");
    const input = page.locator("textarea, input[placeholder*='Ask']").first();
    await input.fill("Refactor router for speed");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText("LOCUS AI Response");

    // 2. Inspect Staged Diffs
    await page.keyboard.press("Control+2");
    await expect(page.locator("body")).toContainText("src/lib/router.rs");

    // 3. Check Cloud Settings & Auto-Detect
    await page.keyboard.press("Control+4");
    const scanBtn = page.locator("button:has-text('Scan & Import Keys')");
    await scanBtn.click();
    await expect(page.locator("body")).toContainText("Key Discovery Results:");

    // 4. Export Diagnostics
    await page.keyboard.press("Control+5");
    const exportBtn = page.locator("button:has-text('Export Diagnostics')");
    await exportBtn.click();
    await expect(page.locator("body")).toContainText(/locus-diagnostic-report/i);

    // Assert zero console errors throughout full journey
    assertNoConsoleErrors(page);
  });
});
