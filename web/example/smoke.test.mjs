// The web example's UI smoke test (docs/standards/TESTING.md): open page 440, click a word,
// and check that the app highlights it. Chromium runs the built example from dist/web.
//
// Run:
//   python3 web/example/build.py dev        # dist/web, with pages/ linked to dist/pages
//   cd web/example && npm ci && npx playwright install chromium && npm test
//
// The test reads the app through the handle it already sets, window.__qvp, and through the
// pixels of the canvas. Without dist/web or dist/pages it skips with a hint. With
// QVP_REQUIRE_DATA set, as in CI, it fails instead.
import { createServer } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../dist/web/', import.meta.url));
for (const need of ['index.html', 'qvp_ffi.wasm', 'pages/440.qvp']) {
  if (existsSync(join(root, need))) continue;
  const what = `dist/web/${need} is missing; run scripts/sync-test-data.sh and python3 web/example/build.py dev`;
  if (process.env.QVP_REQUIRE_DATA) throw new Error(what);
  console.log(`skip ${what}`);
  process.exit(0);
}

// A static server for dist/web. It follows the pages/ link that build.py makes.
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json' };
const server = createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
  const file = join(root, path === '/' ? 'index.html' : path);
  if (!file.startsWith(root) || !existsSync(file) || !statSync(file).isFile()) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { 'content-type': types[extname(file)] || 'application/octet-stream' });
  res.end(readFileSync(file));
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const base = `http://127.0.0.1:${server.address().port}/`;

// The pixels of the canvas that are the selection colour #1a73e8: the highlighted ink, or the
// band over the paper. Light paper and dark ink have blue at most a few levels above red.
// The count, or 0 when it is not above `min`, so that waitForFunction can poll it.
const bluePixels = min => {
  const cv = document.getElementById('cv');
  const d = cv.getContext('2d').getImageData(0, 0, cv.width, cv.height).data;
  let n = 0;
  for (let i = 0; i < d.length; i += 4) if (d[i + 2] - d[i] > 25 && d[i + 2] >= d[i + 1]) n++;
  return n > min ? n : 0;
};

const browser = await chromium.launch();
const errors = [];
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: 'light' });
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(base + '?p=440');
  await page.waitForFunction(() => window.__qvp && window.__qvp.S.n === 440 && window.__qvp.S.page, null, { timeout: 15000 });
  console.log('ok  the example opens page 440');

  const before = await page.evaluate(bluePixels, -1);
  if (before > 50) throw new Error(`the page shows ${before} selection-blue pixels before any click`);

  // Halfway down the page view, right of centre. At this viewport the page fits the view's
  // height, and that point is on the fourth line of Yasin (surah 36), which starts on page 440.
  const stage = page.locator('#stage');
  const box = await stage.boundingBox();
  const at = { x: box.width * 0.7, y: box.height * 0.5 };
  await stage.click({ position: at });

  const word = await page.waitForFunction(() => {
    const { S } = window.__qvp;
    return S.selWord >= 0 && S.page.wordKey(S.selWord);
  }, null, { timeout: 5000 }).then(h => h.jsonValue());
  if (!word.startsWith('36:')) throw new Error(`the click resolved to ${word}, not a word of surah 36`);
  const shown = await page.locator('#selInfo').innerText();
  if (!shown.includes(word)) throw new Error(`the selection panel does not show ${word}: ${shown}`);
  console.log(`ok  the click selects word ${word}, and the panel shows it`);

  const boxes = await page.evaluate(() => window.__qvp.S.page.highlightBoxesView().length);
  if (!boxes) throw new Error('the engine has no highlight boxes after the click');
  // The highlight fades in over the app's transition time; wait for the canvas to show it.
  const after = await page.waitForFunction(bluePixels, 200, { timeout: 5000 }).then(h => h.jsonValue());
  console.log(`ok  highlight boxes from the engine: ${boxes}; selection-blue pixels on the canvas: ${after} (${before} before)`);

  // A click on the same word clears the highlight, as on every platform.
  await stage.click({ position: at });
  await page.waitForFunction(() => window.__qvp.S.selWord === -1, null, { timeout: 5000 });
  console.log('ok  a second click on the word clears the selection');

  if (errors.length) throw new Error(`the page threw: ${errors.join('; ')}`);
} finally {
  await browser.close();
  server.close();
}
