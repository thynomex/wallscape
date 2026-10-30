import { cp, mkdir, mkdtemp, realpath, rm, symlink, unlink } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';
import { installCaptureBridge } from './bridge.mjs';
import { loadFixtures } from './fixtures.mjs';

const directory = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(directory, '../..');
const outputDirectory = join(appRoot, 'docs', 'images');
const rawDirectory = join(directory, '.output');
const port = Number(process.env.WALLSCAPE_SCREENSHOT_PORT ?? 4179);
if (!Number.isInteger(port) || port < 1024 || port > 65535) {
  throw new Error('WALLSCAPE_SCREENSHOT_PORT must be an integer from 1024 to 65535.');
}
const origin = `http://127.0.0.1:${port}`;
const viewport = { width: 1600, height: 832 };
const requireFromApp = createRequire(join(appRoot, 'package.json'));
const { createServer } = await import(pathToFileURL(requireFromApp.resolve('vite')).href);
const fixtures = await loadFixtures(directory);
let previewRoot;
let server;
let browser;

async function createPreviewWorkspace() {
  await mkdir(rawDirectory, { recursive: true });
  // Keep the snapshot on the app's drive: SvelteKit generates relative imports
  // to its installed runtime, and Windows cannot make those across drive letters.
  previewRoot = await mkdtemp(join(rawDirectory, 'preview-'));
  // Snapshot only frontend files. SvelteKit and Vite caches stay in this temporary
  // workspace, separate from other development sessions and the Rust target dir.
  await Promise.all([
    ...['src', 'static'].map((name) => cp(join(appRoot, name), join(previewRoot, name), { recursive: true })),
    ...['package.json', 'tsconfig.json', 'svelte.config.js', 'vite.config.js'].map((name) =>
      cp(join(appRoot, name), join(previewRoot, name))),
  ]);
  await symlink(join(appRoot, 'node_modules'), join(previewRoot, 'node_modules'), process.platform === 'win32' ? 'junction' : 'dir');
}

function assetServer() {
  return {
    name: 'wallscape-screenshot-media',
    configureServer(vite) {
      vite.middlewares.use((request, response, next) => {
        const pathname = new URL(request.url, origin).pathname;
        if (!pathname.startsWith('/__capture_media/')) return next();
        const media = fixtures.media.get(pathname);
        if (!media) {
          response.writeHead(404).end();
          return;
        }
        response.writeHead(200, {
          'Content-Type': media.contentType,
          'Content-Length': media.body.length,
          'Cache-Control': 'no-store',
        });
        response.end(media.body);
      });
    },
  };
}

async function settleImages(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    const visibleImages = [...document.images].filter((image) => {
      const rect = image.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 && rect.top < innerHeight && rect.bottom > 0;
    });
    await Promise.all(visibleImages.map((image) => image.decode()));
  });
  await page.waitForFunction(() => [...document.images].every((image) => {
    const rect = image.getBoundingClientRect();
    const visible = rect.width > 0 && rect.height > 0 && rect.top < innerHeight && rect.bottom > 0;
    return !visible || image.complete && image.naturalWidth > 0;
  }));
}

// Grow the viewport until the scroll container shows the whole view, so long
// views are captured in full instead of cut through a row of cards.
async function fitToContent(page) {
  const steps = [];
  for (let attempt = 0; attempt < 6; attempt += 1) {
    await settleImages(page);
    const overflow = await page.locator('.scroll').evaluate((scroll) => scroll.scrollHeight - scroll.clientHeight);
    const { width, height } = page.viewportSize();
    steps.push(`${height}px viewport: ${overflow}px overflow`);
    if (overflow <= 1) return;
    await page.setViewportSize({ width, height: height + Math.ceil(overflow) });
  }
  throw new Error(`The view kept growing while fitting the viewport to its content:\n${steps.join('\n')}`);
}

async function capture(page, name) {
  await settleImages(page);
  const pngPath = join(rawDirectory, `${name}.png`);
  await page.screenshot({ path: pngPath, animations: 'disabled', caret: 'hide', scale: 'device', omitBackground: true });
  const metadata = await sharp(pngPath).metadata();
  const currentViewport = page.viewportSize();
  if (metadata.width !== currentViewport.width * 2 || metadata.height !== currentViewport.height * 2) {
    throw new Error(`Unexpected dimensions for ${name}: ${metadata.width}x${metadata.height}`);
  }
  const webpPath = join(outputDirectory, `${name}.webp`);
  const info = await sharp(pngPath)
    .flatten({ background: '#090c11' })
    .resize({ width: 2400 })
    .webp({ quality: 88, effort: 6, smartSubsample: true })
    .toFile(webpPath);
  if (info.size > 1024 * 1024) throw new Error(`${name}.webp exceeds the 1 MiB image budget.`);
  console.log(`${name}: ${metadata.width}x${metadata.height}, WebP ${(info.size / 1024).toFixed(0)} KiB`);
}

try {
  await createPreviewWorkspace();
  process.chdir(previewRoot);
  server = await createServer({
    root: previewRoot,
    configFile: join(previewRoot, 'vite.config.js'),
    configLoader: 'native',
    cacheDir: join(previewRoot, '.vite-cache'),
    clearScreen: false,
    logLevel: 'warn',
    plugins: [assetServer()],
    optimizeDeps: {
      include: [
        '@tauri-apps/api/core', '@tauri-apps/api/window',
        '@tauri-apps/api/webview', '@tauri-apps/api/event',
        '@tauri-apps/plugin-dialog',
      ],
    },
    server: { host: '127.0.0.1', port, strictPort: true, hmr: false },
  });
  await server.listen();
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: 2,
    colorScheme: 'dark',
    reducedMotion: 'reduce',
    locale: 'en-US',
    timezoneId: 'UTC',
    serviceWorkers: 'block',
  });
  await context.addInitScript(installCaptureBridge, fixtures.data);
  const failures = [];
  await context.route('**/*', async (route) => {
    const url = new URL(route.request().url());
    if (url.origin === origin || ['data:', 'blob:'].includes(url.protocol)) {
      await route.continue();
    } else {
      failures.push(`Unexpected external request: ${url.origin}${url.pathname}`);
      await route.abort();
    }
  });
  const page = await context.newPage();
  page.setDefaultTimeout(20_000);
  page.on('pageerror', (error) => failures.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') failures.push(message.text());
  });
  await page.goto(origin, { waitUntil: 'networkidle', timeout: 90_000 });
  await page.getByRole('button', { name: `Preview ${fixtures.data.wallpapers[0].title}`, exact: true }).click();
  // Pause the actual carousel through its existing hover behavior. Position the
  // pointer in its padding so no thumbnail gets a hover treatment in the image.
  await page.getByRole('group', { name: 'Featured wallpaper carousel' }).hover({ position: { x: 8, y: 8 } });
  const carouselClearance = await page.locator('.thumbnail-carousel').evaluate((container) => {
    const clip = container.getBoundingClientRect();
    const active = container.querySelector('.thumbnail-item.active').getBoundingClientRect();
    return { top: active.top - clip.top, bottom: clip.bottom - active.bottom };
  });
  if (carouselClearance.top < 6 || carouselClearance.bottom < 6) {
    throw new Error(`The featured thumbnail outline is clipped: ${JSON.stringify(carouselClearance)}`);
  }
  console.log(`Active thumbnail clearance: ${carouselClearance.top.toFixed(1)}px above, ${carouselClearance.bottom.toFixed(1)}px below.`);
  await Promise.all([mkdir(outputDirectory, { recursive: true }), mkdir(rawDirectory, { recursive: true })]);
  await capture(page, 'home');

  await page.setViewportSize({ width: 1600, height: 1200 });
  await page.getByRole('button', { name: 'Library', exact: true }).click();
  await page.locator('.library-results .wallpaper-card').last().waitFor({ state: 'visible' });
  if (await page.locator('.library-results .wallpaper-card').count() !== fixtures.data.wallpapers.length) {
    throw new Error('The library did not render every fixture wallpaper.');
  }
  await page.locator('.monitor-bar').scrollIntoViewIfNeeded();
  await page.waitForFunction(() => {
    const viewport = document.querySelector('.scroll').getBoundingClientRect();
    const footer = document.querySelector('.monitor-bar').getBoundingClientRect();
    const cards = [...document.querySelectorAll('.library-results .wallpaper-card')];
    return footer.bottom <= viewport.bottom + 1 && cards.every((card) => {
      const rect = card.getBoundingClientRect();
      return rect.top >= viewport.top && rect.bottom <= viewport.bottom;
    });
  });
  await page.mouse.move(1, 1);
  await capture(page, 'library');

  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.getByRole('button', { name: `Set ${fixtures.data.wallpapers[0].title} as wallpaper`, exact: true }).click();
  await page.getByLabel('Wallpaper target display').selectOption('capture-primary');
  await page.getByRole('button', { name: 'Set Wallpaper', exact: true }).waitFor({ state: 'visible' });
  await page.mouse.move(1, 1);
  await capture(page, 'wallpaper-detail');
  await page.getByRole('button', { name: 'Close preview', exact: true }).click();
  await page.getByRole('button', { name: 'Close preview', exact: true }).waitFor({ state: 'detached' });

  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.getByRole('button', { name: 'Discover', exact: true }).click();
  await page.locator('.discover-grid .discover-card').nth(fixtures.data.wallhaven.data.length - 1).waitFor({ state: 'visible' });
  await fitToContent(page);
  await page.mouse.move(1, 1);
  await capture(page, 'discover');

  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.getByRole('button', { name: 'MotionBGS', exact: true }).click();
  await page.locator('.discover-card .quality-selector').nth(fixtures.data.motionBgs.data.length - 1).waitFor({ state: 'visible' });
  await fitToContent(page);
  await page.mouse.move(1, 1);
  await capture(page, 'discover-motionbgs');

  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.getByRole('button', { name: 'Explore', exact: true }).click();
  await page.getByPlaceholder('Search wallpapers...').fill('Mountains');
  await page.getByPlaceholder('Search wallpapers...').press('Enter');
  await page.waitForFunction(() =>
    window.__WALLSCAPE_CAPTURE__.calls.includes('search_wallpapers') &&
    !document.querySelector('.loading-grid') &&
    document.querySelectorAll('.card-grid .wallpaper-card').length > 0);
  await fitToContent(page);
  await page.mouse.move(1, 1);
  await capture(page, 'explore');

  await page.setViewportSize({ width: 1600, height: 1000 });
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('dialog', { name: 'Settings' }).waitFor({ state: 'visible' });
  await page.waitForFunction(() => {
    const values = [...document.querySelectorAll('.storage-metrics .metric-value')];
    return values.length > 0 && values.every((value) => value.textContent.trim() !== '-');
  });
  await page.mouse.move(1, 1);
  await capture(page, 'settings');

  const bridge = await page.evaluate(() => window.__WALLSCAPE_CAPTURE__);
  if (bridge.unexpected.length) failures.push(`Unexpected IPC: ${bridge.unexpected.join(', ')}`);
  if (failures.length) throw new Error([...new Set(failures)].join('\n'));
  console.log('Captured the real frontend with isolated sample data. No native app or wallpaper commands ran.');
} finally {
  await browser?.close();
  await server?.close();
  process.chdir(appRoot);
  if (previewRoot) {
    const actualRoot = await realpath(previewRoot);
    const actualOutput = await realpath(rawDirectory);
    if (dirname(actualRoot) !== actualOutput || !basename(actualRoot).startsWith('preview-')) {
      throw new Error(`Refusing to clean an unexpected preview directory: ${actualRoot}`);
    }
    // Remove the dependency link before recursively deleting our temp snapshot.
    await unlink(join(actualRoot, 'node_modules')).catch((error) => {
      if (error.code !== 'ENOENT') throw error;
    });
    await rm(actualRoot, { recursive: true, maxRetries: 3, retryDelay: 100 });
  }
}
