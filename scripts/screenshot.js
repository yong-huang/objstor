const { chromium } = require('playwright');

(async () => {
    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
        locale: 'en-US',
    });
    const page = await context.newPage();
    const BASE = 'http://localhost:3020';

    // --- 1. Dashboard ---
    console.log('Capturing dashboard...');
    await page.goto(`${BASE}/web`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'screenshots/dashboard.png', fullPage: false });
    console.log('  -> dashboard.png');

    // --- 2. Buckets ---
    console.log('Capturing buckets...');
    await page.click('.nav-item[data-page="buckets"]');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'screenshots/buckets.png', fullPage: false });
    console.log('  -> buckets.png');

    // --- 3. Objects (with bucket selected) ---
    console.log('Capturing objects...');
    await page.click('.nav-item[data-page="objects"]');
    await page.waitForTimeout(1500);
    // Select a bucket that has objects
    const bucketSelector = page.locator('#bucket-selector');
    await bucketSelector.waitFor({ state: 'visible' });
    await bucketSelector.selectOption('my-bucket');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'screenshots/objects.png', fullPage: false });
    console.log('  -> objects.png');

    // --- 4. Auto-Tagging (objects page with tag buttons visible) ---
    console.log('Capturing auto-tag...');
    // Make sure we're on objects page with my-bucket selected
    await page.waitForTimeout(500);
    await page.screenshot({ path: 'screenshots/auto-tag.png', fullPage: false });
    console.log('  -> auto-tag.png');

    // --- 5. Content Summarization (same objects page) ---
    console.log('Capturing summarize...');
    await page.screenshot({ path: 'screenshots/summarize.png', fullPage: false });
    console.log('  -> summarize.png');

    // --- 6. AI Chat Assistant ---
    console.log('Capturing AI chat...');
    await page.click('.nav-item[data-page="dashboard"]');
    await page.waitForTimeout(1000);
    // Clear any existing chat history for a clean screenshot
    await page.evaluate(() => {
        localStorage.removeItem('ai-chat-history');
    });
    // Click the FAB to open chat panel
    await page.click('.ai-chat-fab');
    await page.waitForTimeout(500);

    // Type a message and wait for response
    const chatInput = page.locator('#ai-chat-input');
    await chatInput.fill('How many buckets and objects do I have?');
    await page.waitForTimeout(300);
    await chatInput.press('Enter');
    await page.waitForTimeout(8000); // Wait for LLM response

    await page.screenshot({ path: 'screenshots/ai-chat.png', fullPage: false });
    console.log('  -> ai-chat.png');

    // Close chat panel via JS
    await page.evaluate(() => {
        document.getElementById('ai-chat-panel')?.classList.remove('open');
        document.body.classList.remove('ai-chat-open');
    });
    await page.waitForTimeout(500);

    // --- 7. Lifecycle Suggestions ---
    console.log('Capturing lifecycle suggestions...');
    await page.waitForTimeout(1000);
    // The lifecycle card is hidden by default; show it and trigger analysis
    await page.evaluate(() => {
        document.getElementById('lifecycle-suggestions-card').style.display = '';
    });
    await page.waitForTimeout(300);
    // Click Refresh button inside the card
    const refreshBtn = page.locator('#lifecycle-suggestions-card button');
    if (await refreshBtn.isVisible()) {
        await refreshBtn.click();
        console.log('  Waiting for lifecycle analysis...');
        await page.waitForTimeout(12000); // Wait for AI lifecycle analysis
    }
    await page.screenshot({ path: 'screenshots/lifecycle.png', fullPage: false });
    console.log('  -> lifecycle.png');

    // --- 8. Settings with AI Configuration ---
    console.log('Capturing settings...');
    await page.click('.nav-item[data-page="settings"]');
    await page.waitForTimeout(1000);
    await page.screenshot({ path: 'screenshots/settings.png', fullPage: false });
    console.log('  -> settings.png');

    // --- 9. Dark theme dashboard ---
    console.log('Capturing dark theme...');
    await page.click('#theme-toggle');
    await page.waitForTimeout(500);
    await page.click('.nav-item[data-page="dashboard"]');
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'screenshots/dashboard-dark.png', fullPage: false });
    console.log('  -> dashboard-dark.png');

    await browser.close();
    console.log('Done! All screenshots saved to screenshots/');
})();
