const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const messages = [];
  page.on('console', msg => {
    messages.push({ type: msg.type(), text: msg.text() });
  });
  page.on('pageerror', err => {
    messages.push({ type: 'pageerror', text: err.message });
  });
  page.on('requestfailed', request => {
    messages.push({ type: 'requestfailed', text: `${request.method()} ${request.url()} ${request.failure()?.errorText}` });
  });

  try {
    await page.goto('http://localhost:1420', { waitUntil: 'domcontentloaded', timeout: 15000 });
    await page.waitForTimeout(2000);
    const title = await page.title();
    const html = await page.evaluate(() => document.documentElement.outerHTML);
    console.log('PAGE_TITLE:', title);
    console.log('HTML_START:', html.slice(0, 1200).replace(/\n/g, ' '));
    console.log('BODY_TEXT:', await page.evaluate(() => document.body.innerText.slice(0, 800)));
  } catch (e) {
    console.error('ERROR:', e.stack || e);
  } finally {
    console.log('CONSOLE_LOGS:', JSON.stringify(messages, null, 2));
    await browser.close();
  }
})();