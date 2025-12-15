import { test, expect } from '@playwright/test';

test.describe('Example Tests', () => {
  test('has title', async ({ page }) => {
    await page.goto('https://example.com');
    await expect(page).toHaveTitle(/Example Domain/);
  });

  test('contains example heading', async ({ page }) => {
    await page.goto('https://example.com');
    const heading = page.locator('h1');
    await expect(heading).toContainText('Example Domain');
  });

  test('has more information link', async ({ page }) => {
    await page.goto('https://example.com');
    const link = page.locator('a');
    await expect(link).toHaveAttribute('href', 'https://iana.org/domains/example');
  });
});
