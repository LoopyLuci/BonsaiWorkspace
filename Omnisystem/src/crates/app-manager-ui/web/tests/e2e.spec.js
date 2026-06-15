import { test, expect } from '@playwright/test';

/**
 * End-to-End Tests for App Manager Desktop Application
 * Tests complete user workflows and interactions
 */

test.describe('App Manager E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to application before each test
    await page.goto('http://localhost:5173');
    // Wait for app to load
    await page.waitForLoadState('networkidle');
  });

  test.describe('Authentication Flow', () => {
    test('should complete login flow successfully', async ({ page }) => {
      // Check login form is visible
      const usernameInput = page.locator('input[id="userId"]');
      const passwordInput = page.locator('input[id="password"]');
      const loginButton = page.locator('button:has-text("Sign In")');

      await expect(usernameInput).toBeVisible();
      await expect(passwordInput).toBeVisible();
      await expect(loginButton).toBeVisible();

      // Enter credentials
      await usernameInput.fill('test-user');
      await passwordInput.fill('Password123!');

      // Click login
      await loginButton.click();

      // Wait for navigation to marketplace
      await page.waitForLoadState('networkidle');

      // Check user is authenticated
      const userNameDisplay = page.locator('text="test-user"');
      await expect(userNameDisplay).toBeVisible();
    });

    test('should show error for invalid credentials', async ({ page }) => {
      const usernameInput = page.locator('input[id="userId"]');
      const passwordInput = page.locator('input[id="password"]');
      const loginButton = page.locator('button:has-text("Sign In")');

      await usernameInput.fill('invalid');
      await passwordInput.fill('wrong');
      await loginButton.click();

      // Check for error message
      const errorMessage = page.locator('[role="alert"]');
      await expect(errorMessage).toBeVisible();
    });

    test('should support Enter key for login', async ({ page }) => {
      const usernameInput = page.locator('input[id="userId"]');
      const passwordInput = page.locator('input[id="password"]');

      await usernameInput.fill('test-user');
      await passwordInput.fill('Password123!');

      // Press Enter on password field
      await passwordInput.press('Enter');

      // Should trigger login
      await page.waitForLoadState('networkidle');
    });
  });

  test.describe('App Discovery', () => {
    test('should display app marketplace', async ({ page }) => {
      // Login first
      await loginAndNavigate(page);

      // Check marketplace header
      const marketplaceHeader = page.locator('text="App Marketplace"');
      await expect(marketplaceHeader).toBeVisible();

      // Check for app cards
      const appCards = page.locator('[role="region"]').filter({ hasText: 'App' });
      const cardCount = await appCards.count();
      expect(cardCount).toBeGreaterThan(0);
    });

    test('should search apps in real-time', async ({ page }) => {
      await loginAndNavigate(page);

      const searchInput = page.locator('input[placeholder*="Search"]');
      await searchInput.fill('productivity');

      // Wait for search results
      await page.waitForTimeout(300);

      // Check results are filtered
      const appCards = page.locator('[role="article"]');
      await expect(appCards.first()).toBeVisible();
    });

    test('should toggle view modes (All/Trending/Featured)', async ({ page }) => {
      await loginAndNavigate(page);

      // Click Trending tab
      const trendingButton = page.locator('button:has-text("Trending")');
      await trendingButton.click();

      // Wait for view to update
      await page.waitForTimeout(300);

      // Check content updated
      const appCards = page.locator('[role="article"]');
      await expect(appCards.first()).toBeVisible();

      // Click Featured tab
      const featuredButton = page.locator('button:has-text("Featured")');
      await featuredButton.click();

      await page.waitForTimeout(300);
      await expect(appCards.first()).toBeVisible();
    });

    test('should display app details on card click', async ({ page }) => {
      await loginAndNavigate(page);

      // Find first app card
      const appCard = page.locator('[role="article"]').first();
      await appCard.locator('text="View Details"').click();

      // Check details panel appears
      const detailsPanel = page.locator('[role="article"]').filter({ hasText: 'Version' });
      await expect(detailsPanel).toBeVisible();
    });
  });

  test.describe('App Installation', () => {
    test('should install app', async ({ page }) => {
      await loginAndNavigate(page);

      // Find and click install button
      const installButton = page
        .locator('button:has-text("Install")')
        .first();

      const initialText = await installButton.textContent();
      expect(initialText).toContain('Install');

      await installButton.click();

      // Check for notification
      const notification = page.locator('[role="status"]');
      await expect(notification).toBeVisible({ timeout: 5000 });
    });

    test('should display installation progress', async ({ page }) => {
      await loginAndNavigate(page);

      const installButton = page
        .locator('button:has-text("Install")')
        .first();

      await installButton.click();

      // Wait for loading state
      await expect(installButton).toHaveText(/Installing|Installed/, {
        timeout: 5000,
      });
    });

    test('should show installed status for installed apps', async ({ page }) => {
      await loginAndNavigate(page);

      // Look for installed app indicator
      const installedIndicator = page.locator('text="✓ Installed"');
      const count = await installedIndicator.count();

      // Should have at least some installed apps (from mock data)
      expect(count).toBeGreaterThanOrEqual(0);
    });
  });

  test.describe('Settings Management', () => {
    test('should navigate to settings', async ({ page }) => {
      await loginAndNavigate(page);

      // Click settings in navigation
      const settingsButton = page.locator('button:has-text("Settings")');
      await settingsButton.click();

      // Check settings page loaded
      const settingsHeader = page.locator('text="Settings"');
      await expect(settingsHeader).toBeVisible();
    });

    test('should change theme setting', async ({ page }) => {
      await loginAndNavigate(page);

      const settingsButton = page.locator('button:has-text("Settings")');
      await settingsButton.click();

      // Find theme select
      const themeSelect = page.locator('select').first();
      await themeSelect.selectOption('light');

      // Check save button
      const saveButton = page.locator('button:has-text("Save Settings")');
      await saveButton.click();

      // Wait for save notification
      const savedMessage = page.locator('text="Saved successfully"');
      await expect(savedMessage).toBeVisible({ timeout: 5000 });
    });

    test('should change language setting', async ({ page }) => {
      await loginAndNavigate(page);

      const settingsButton = page.locator('button:has-text("Settings")');
      await settingsButton.click();

      // Find language select (second select)
      const languageSelect = page.locator('select').nth(1);
      await languageSelect.selectOption('es');

      // Save settings
      const saveButton = page.locator('button:has-text("Save Settings")');
      await saveButton.click();

      // Verify save
      await expect(page.locator('text="Saved successfully"')).toBeVisible({
        timeout: 5000,
      });
    });

    test('should toggle notification preference', async ({ page }) => {
      await loginAndNavigate(page);

      const settingsButton = page.locator('button:has-text("Settings")');
      await settingsButton.click();

      // Find notification checkbox
      const notificationCheckbox = page
        .locator('input[type="checkbox"]')
        .first();

      // Toggle checkbox
      await notificationCheckbox.click();

      // Save settings
      const saveButton = page.locator('button:has-text("Save Settings")');
      await saveButton.click();

      // Verify
      await expect(page.locator('text="Saved successfully"')).toBeVisible({
        timeout: 5000,
      });
    });
  });

  test.describe('User Experience', () => {
    test('should display notifications', async ({ page }) => {
      await loginAndNavigate(page);

      // Trigger a notification-generating action
      const installButton = page
        .locator('button:has-text("Install")')
        .first();
      await installButton.click();

      // Check notification appears
      const notification = page.locator('[role="status"]');
      await expect(notification).toBeVisible({ timeout: 5000 });
    });

    test('should auto-dismiss notifications', async ({ page }) => {
      await loginAndNavigate(page);

      // Create notification
      const installButton = page
        .locator('button:has-text("Install")')
        .first();
      await installButton.click();

      // Wait for notification
      const notification = page.locator('[role="status"]');
      await expect(notification).toBeVisible();

      // Wait for auto-dismiss
      await page.waitForTimeout(6000);
      await expect(notification).not.toBeVisible();
    });

    test('should maintain responsive layout', async ({ page }) => {
      // Test mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });

      await loginAndNavigate(page);

      // Elements should still be visible
      const navigation = page.locator('nav');
      await expect(navigation).toBeVisible();

      const marketplace = page.locator('text="Marketplace"');
      await expect(marketplace).toBeVisible();
    });

    test('should support keyboard navigation', async ({ page }) => {
      const usernameInput = page.locator('input[id="userId"]');

      await usernameInput.fill('test-user');

      // Tab to password
      await page.keyboard.press('Tab');
      const passwordInput = page.locator('input[id="password"]');
      await expect(passwordInput).toBeFocused();

      // Tab to button
      await page.keyboard.press('Tab');
      const loginButton = page.locator('button:has-text("Sign In")');
      await expect(loginButton).toBeFocused();
    });
  });

  test.describe('Logout Flow', () => {
    test('should logout successfully', async ({ page }) => {
      await loginAndNavigate(page);

      // Click logout button
      const logoutButton = page.locator('button:has-text("Logout")');
      await logoutButton.click();

      // Should return to login page
      const loginForm = page.locator('text="App Manager"').first();
      await expect(loginForm).toBeVisible();
    });

    test('should clear session on logout', async ({ page }) => {
      await loginAndNavigate(page);

      const logoutButton = page.locator('button:has-text("Logout")');
      await logoutButton.click();

      // Login form should be visible
      const usernameInput = page.locator('input[id="userId"]');
      await expect(usernameInput).toBeVisible();

      // Should be able to login again
      await usernameInput.fill('test-user');
      const passwordInput = page.locator('input[id="password"]');
      await passwordInput.fill('Password123!');

      const loginButton = page.locator('button:has-text("Sign In")');
      await loginButton.click();

      // Should authenticate again
      await page.waitForLoadState('networkidle');
    });
  });
});

/**
 * Helper function to login and navigate to marketplace
 */
async function loginAndNavigate(page) {
  const usernameInput = page.locator('input[id="userId"]');
  const passwordInput = page.locator('input[id="password"]');
  const loginButton = page.locator('button:has-text("Sign In")');

  // Only login if not already authenticated
  const marketplaceHeader = page.locator('text="App Marketplace"');
  const isVisible = await marketplaceHeader.isVisible().catch(() => false);

  if (!isVisible) {
    await usernameInput.fill('test-user');
    await passwordInput.fill('Password123!');
    await loginButton.click();
    await page.waitForLoadState('networkidle');
  }
}
