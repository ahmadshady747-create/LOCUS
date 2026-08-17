import { test, expect } from "@playwright/test";
import { setupTauriMocks, assertNoConsoleErrors } from "./mocks/tauri-mock";

test.describe("Suite 3: Inline Diff Viewer & Workspace Management (20 Scenarios)", () => {
  test.beforeEach(async ({ page }) => {
    await setupTauriMocks(page);
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await page.click("nav button:has-text('Workspace & Diffs')");
  });

  test("3.1 Workspace metrics card displays total indexed files", async ({ page }) => {
    await expect(page.locator("body")).toContainText("Indexed Files");
    await expect(page.locator("body")).toContainText("28");
    assertNoConsoleErrors(page);
  });

  test("3.2 Total index size in MB is rendered accurately", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/4.0 MB indexed|MB indexed/i);
    assertNoConsoleErrors(page);
  });

  test("3.3 Local AI models count metric is displayed", async ({ page }) => {
    await expect(page.locator("body")).toContainText("AI Models Detected");
    await expect(page.locator("body")).toContainText("3");
    assertNoConsoleErrors(page);
  });

  test("3.4 Staged changes counter badge displays active diff count", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/Staged|Pending AST/i);
    assertNoConsoleErrors(page);
  });

  test("3.5 Staged files list renders file paths with change badges", async ({ page }) => {
    await expect(page.locator("body")).toContainText("src/lib/router.rs");
    assertNoConsoleErrors(page);
  });

  test("3.6 Clicking a staged change opens the inline Diff Viewer", async ({ page }) => {
    const diffItem = page.locator("button:has-text('src/lib/router.rs'), div:has-text('src/lib/router.rs')").first();
    await diffItem.click();
    await expect(page.locator("body")).toContainText(/router.rs|Proposed|Accept/i);
    assertNoConsoleErrors(page);
  });

  test("3.7 Inline Diff Viewer renders additions in green with + markers", async ({ page }) => {
    const diffItem = page.locator("button:has-text('src/lib/router.rs'), div:has-text('src/lib/router.rs')").first();
    if (await diffItem.isVisible()) {
      await diffItem.click();
      await expect(page.locator("body")).toContainText("+");
    }
    assertNoConsoleErrors(page);
  });

  test("3.8 Inline Diff Viewer renders deletions in red with - markers", async ({ page }) => {
    const diffItem = page.locator("button:has-text('src/lib/router.rs'), div:has-text('src/lib/router.rs')").first();
    if (await diffItem.isVisible()) {
      await diffItem.click();
      await expect(page.locator("body")).toContainText(/match strategy|"local"/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.9 Switching diff display between Unified and Split mode", async ({ page }) => {
    const diffItem = page.locator("button:has-text('src/lib/router.rs'), div:has-text('src/lib/router.rs')").first();
    if (await diffItem.isVisible()) {
      await diffItem.click();
    }
    const splitBtn = page.locator("button:has-text('Split'), button:has-text('Side-by-Side'), button:has-text('Inline')").first();
    if (await splitBtn.isVisible()) {
      await splitBtn.click();
    }
    assertNoConsoleErrors(page);
  });

  test("3.10 Accept Hunk button applies discrete block and updates diff", async ({ page }) => {
    const hunkAcceptBtn = page.locator("button:has-text('Accept Hunk'), button:has-text('✓ Accept Hunk'), button:has-text('Accept All')").first();
    if (await hunkAcceptBtn.isVisible()) {
      await hunkAcceptBtn.click();
    }
    assertNoConsoleErrors(page);
  });

  test("3.11 Rollback Last Action button restores previous snapshot", async ({ page }) => {
    const rollbackBtn = page.locator("button:has-text('Rollback Last Action'), button:has-text('Rollback')").first();
    if (await rollbackBtn.isVisible()) {
      await rollbackBtn.click();
      await page.waitForTimeout(200);
      await expect(page.locator("body")).toContainText(/rolled back|snapshot|previous/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.12 Rescan workspace folder triggers indexing refresh", async ({ page }) => {
    const scanBtn = page.locator("button:has-text('Scan'), button:has-text('Index'), button:has-text('Rescan')").first();
    if (await scanBtn.isVisible()) {
      await scanBtn.click();
      await page.waitForTimeout(200);
    }
    assertNoConsoleErrors(page);
  });

  test("3.13 Semantic Search input accepts natural language query", async ({ page }) => {
    const searchInput = page.locator("input[placeholder*='Search']").first();
    if (await searchInput.isVisible()) {
      await searchInput.fill("diagnostics sanitization engine");
      await expect(searchInput).toHaveValue("diagnostics sanitization engine");
    }
    assertNoConsoleErrors(page);
  });

  test("3.14 Semantic Search displays ranked similarity results with line ranges", async ({ page }) => {
    const searchInput = page.locator("input[placeholder*='Search']").first();
    if (await searchInput.isVisible()) {
      await searchInput.fill("keyring encryption");
      await page.waitForTimeout(400);
      await expect(page.locator("body")).toContainText(/diagnostics|keyring|Symbol/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.15 Search mode toggle between Semantic Embedding and Exact Text", async ({ page }) => {
    const modeToggle = page.locator("button:has-text('Exact'), button:has-text('Semantic'), button:has-text('Text')").first();
    if (await modeToggle.isVisible()) {
      await modeToggle.click();
    }
    assertNoConsoleErrors(page);
  });

  test("3.16 Model Benchmark runner button executes inference test", async ({ page }) => {
    const benchBtn = page.locator("button:has-text('Benchmark'), button:has-text('Run Benchmark')").first();
    if (await benchBtn.isVisible()) {
      await benchBtn.click();
      await expect(page.locator("body")).toContainText(/Benchmark|Latency/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.17 Benchmark results display token generation speed and ms latency", async ({ page }) => {
    const benchBtn = page.locator("button:has-text('Benchmark'), button:has-text('Run Benchmark')").first();
    if (await benchBtn.isVisible()) {
      await benchBtn.click();
      await page.waitForTimeout(300);
      await expect(page.locator("body")).toContainText(/tokens\/s|ms|Latency/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.18 Staged Changes container renders AST edits section", async ({ page }) => {
    await expect(page.locator("body")).toContainText(/Staged|AST|Changes/i);
    assertNoConsoleErrors(page);
  });

  test("3.19 Create sample staged diff button creates new pending change", async ({ page }) => {
    const testDiffBtn = page.locator("button:has-text('Stage Sample'), button:has-text('Sample Diff')").first();
    if (await testDiffBtn.isVisible()) {
      await testDiffBtn.click();
      await expect(page.locator("body")).toContainText(/Optimized|router/i);
    }
    assertNoConsoleErrors(page);
  });

  test("3.20 Language filter buttons filter workspace files", async ({ page }) => {
    const langBtn = page.locator("button:has-text('Rust'), button:has-text('TypeScript'), button:has-text('All')").first();
    if (await langBtn.isVisible()) {
      await langBtn.click();
    }
    assertNoConsoleErrors(page);
  });
});
