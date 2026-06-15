import { test, expect } from '@playwright/test';

const APP_URL = 'http://localhost:1420';

async function waitForServer(url: string, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch { /* not ready */ }
    await new Promise(r => setTimeout(r, 500));
  }
  throw new Error(`Server at ${url} did not start within ${timeoutMs}ms`);
}

test.describe('Bonsai Critical Path', () => {
  test.beforeAll(async () => {
    await waitForServer(APP_URL, 30000);
  });

  test('MLP-01: App launches and shows model status', async ({ page }) => {
    await page.goto(APP_URL);
    await expect(page.locator('[data-bonsai-action="Model:Select"]')).toBeVisible({ timeout: 10000 });
  });

  test('MLP-02: File tree container is present', async ({ page }) => {
    await page.goto(APP_URL);
    await expect(page.locator('[data-bonsai-action="FileTree"]')).toBeVisible();
  });

  test('MLP-03: Chat message receives a response', async ({ page }) => {
    await page.goto(APP_URL);
    const input = page.locator('[data-bonsai-action="Chat:Send"] textarea');
    await input.fill('Hello');
    await page.locator('[data-bonsai-action="Chat:Send"] button').click();
    await expect(page.locator('.message-bubble.assistant')).toBeVisible({ timeout: 30000 });
  });

  test('MLP-04: Session persistence across restart', async ({ page }) => {
    await page.goto(APP_URL);
    const msg = `persist-test-${Date.now()}`;
    const input = page.locator('[data-bonsai-action="Chat:Send"] textarea');
    await input.fill(msg);
    await page.locator('[data-bonsai-action="Chat:Send"] button').click();
    await expect(page.locator('.message-bubble.assistant')).toBeVisible({ timeout: 30000 });
    await page.reload({ waitUntil: 'domcontentloaded' });
    await expect(page.locator(`text="${msg}"`)).toBeVisible({ timeout: 10000 });
  });
});
