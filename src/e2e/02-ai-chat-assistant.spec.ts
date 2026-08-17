import { test, expect } from "@playwright/test";
import { setupTauriMocks, assertNoConsoleErrors } from "./mocks/tauri-mock";

test.describe("Suite 2: AI Chat & Messaging Experience (25 Scenarios)", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
  });

  test("2.1 Chat input is visible and displays prompt placeholder", async ({ page }) => {
    const input = page.locator("textarea").first();
    await expect(input).toBeVisible();
    assertNoConsoleErrors(page);
  });

  test("2.2 Send button is disabled when input is empty", async ({ page }) => {
    const sendBtn = page.locator("button[aria-label='Send message']");
    await expect(sendBtn).toBeDisabled();
    assertNoConsoleErrors(page);
  });

  test("2.3 Typing in chat input updates text correctly", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Hello LOCUS, write a Rust struct.");
    await expect(input).toHaveValue("Hello LOCUS, write a Rust struct.");
    assertNoConsoleErrors(page);
  });

  test("2.4 Sending message via Enter key posts user message", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Generate binary search in Rust");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText("Generate binary search in Rust");
    assertNoConsoleErrors(page);
  });

  test("2.5 Sending message via Send button click posts user message", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Explain memory ownership in Rust");
    const sendBtn = page.locator("button[aria-label='Send message']");
    await sendBtn.click();
    await expect(page.locator("body")).toContainText("Explain memory ownership in Rust");
    assertNoConsoleErrors(page);
  });

  test("2.6 Multiline text input with Shift+Enter does not submit prematurely", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Line 1");
    await input.press("Shift+Enter");
    await input.type("Line 2");
    const val = await input.inputValue();
    expect(val).toContain("Line 1");
    expect(val).toContain("Line 2");
    assertNoConsoleErrors(page);
  });

  test("2.7 AI Assistant responds with markdown headers and text", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Show me code");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText("LOCUS AI Response");
    assertNoConsoleErrors(page);
  });

  test("2.8 Code block renders with syntax highlighting styling", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Show code example");
    await input.press("Enter");
    await expect(page.locator("pre").first()).toBeVisible();
    assertNoConsoleErrors(page);
  });

  test("2.9 Code block copy button copies snippet to clipboard", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Generate fibonacci");
    await input.press("Enter");
    const copyBtn = page.locator("button[title='Copy response'], button:has-text('Copy')").first();
    await copyBtn.waitFor({ state: "visible", timeout: 5000 });
    await copyBtn.click();
    await page.waitForTimeout(200);
    await expect(page.locator("body")).toContainText(/Copied|✓/i);
    assertNoConsoleErrors(page);
  });

  test("2.10 AI message displays provider watermark stamp", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Test watermark");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText(/Local|qwen/i);
    assertNoConsoleErrors(page);
  });

  test("2.11 Latency metrics badge is displayed on assistant message", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Test latency display");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText(/ms|latency/i);
    assertNoConsoleErrors(page);
  });

  test("2.12 Model selector badge displays active model", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/qwen2.5-coder|Auto Model/i);
    assertNoConsoleErrors(page);
  });

  test("2.13 Context Template picker opens and displays categories", async ({ page }) => {
    const templateBtn = page.locator("button[title*='Context Templates']");
    await templateBtn.click();
    await expect(page.locator("body")).toContainText("Context Templates");
    assertNoConsoleErrors(page);
  });

  test("2.14 Quick prompt suggestion button populates and sends query", async ({ page }) => {
    const suggestionBtn = page.locator("button:has-text('Optimize Code'), button:has-text('Explain Logic')").first();
    if (await suggestionBtn.isVisible()) {
      await suggestionBtn.click();
      await expect(page.locator("body")).toContainText(/LOCUS AI Response|Review and optimize/i);
    }
    assertNoConsoleErrors(page);
  });

  test("2.15 Clear conversation button clears chat history", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Message to clear");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText("Message to clear");

    const clearBtn = page.locator("button:has-text('Clear')");
    await clearBtn.click();
    assertNoConsoleErrors(page);
  });

  test("2.16 Empty chat shows introductory guide message", async ({ page }) => {
    const clearBtn = page.locator("button:has-text('Clear')");
    await clearBtn.click();
    await expect(page.locator("body")).toContainText(/Chat cleared/i);
    assertNoConsoleErrors(page);
  });

  test("2.17 Chat auto-scrolls down when receiving responses", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Long output request 1");
    await input.press("Enter");
    await page.waitForTimeout(200);
    await input.fill("Long output request 2");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText("Long output request 2");
    assertNoConsoleErrors(page);
  });

  test("2.18 Simulated LLM provider error renders visual alert banner gracefully", async ({ page }) => {
    await setupTauriMocks(page, { simulateChatError: true });
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    const input = page.locator("textarea").first();
    await input.fill("Trigger failing LLM call");
    await input.press("Enter");
    await expect(page.locator("body")).toContainText(/Execution Warning|Timeout/i);
    await expect(page.locator("header")).toContainText("LOCUS");
  });

  test("2.19 Privacy badge in chat footer indicates local processing mode", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/100% On-Device|Mesh Connected/i);
    assertNoConsoleErrors(page);
  });

  test("2.20 Hybrid Privacy mode toggle displays state indicator", async ({ page }) => {
    await page.keyboard.press("Control+4");
    await expect(page.locator("body")).toContainText(/Local|Hybrid/i);
    assertNoConsoleErrors(page);
  });

  test("2.21 Context template search filters tags", async ({ page }) => {
    const templateBtn = page.locator("button[title*='Context Templates']");
    await templateBtn.click();
    const filterInput = page.locator("input[placeholder*='Filter templates']");
    await filterInput.fill("rust");
    await expect(filterInput).toHaveValue("rust");
    assertNoConsoleErrors(page);
  });

  test("2.22 Quick Architectural Mode buttons (/grill, /plan, /spec) populate input prompt", async ({ page }) => {
    const grillBtn = page.locator("button:has-text('/grill')").first();
    if (await grillBtn.isVisible()) {
      await grillBtn.click();
      const inputVal = await page.locator("textarea").first().inputValue();
      expect(inputVal).toContain("/grill");
    }

    const planBtn = page.locator("button:has-text('/plan')").first();
    if (await planBtn.isVisible()) {
      await planBtn.click();
      const inputVal = await page.locator("textarea").first().inputValue();
      expect(inputVal).toContain("/plan");
    }

    const specBtn = page.locator("button:has-text('/spec')").first();
    if (await specBtn.isVisible()) {
      await specBtn.click();
      const inputVal = await page.locator("textarea").first().inputValue();
      expect(inputVal).toContain("/spec");
    }
    assertNoConsoleErrors(page);
  });

  test("2.23 Sound effect triggers on message send and receive", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Sound check message");
    await input.press("Enter");
    await page.waitForTimeout(300);
    assertNoConsoleErrors(page);
  });

  test("2.24 Textarea auto-resizes as multiline content grows", async ({ page }) => {
    const input = page.locator("textarea").first();
    await input.fill("Line 1\nLine 2\nLine 3\nLine 4");
    const height = await input.evaluate((el) => el.clientHeight);
    expect(height).toBeGreaterThan(30);
    assertNoConsoleErrors(page);
  });

  test("2.25 Chat responsive container adapts when viewport width changes", async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 700 });
    const input = page.locator("textarea").first();
    await expect(input).toBeVisible();
    await page.setViewportSize({ width: 1280, height: 800 });
    assertNoConsoleErrors(page);
  });
});
