// Smoke test: every public entry point loads and the engine answers.
// Drawing is not exercised — buildPaths() and drawBoxes() need a DOM Path2D.
import { readFileSync } from 'node:fs';
import { QvpEngine, wasmUrl } from './index.mjs';

const engine = await QvpEngine.init(readFileSync(wasmUrl));
const strip = engine.strip('الْحَمْدُ');
if (!strip) throw new Error('engine.strip() returned nothing');

console.log('ok  wasm instantiated, engine.strip() ->', strip);
if (engine.engineName() !== 'qvp' || !/^\d+\.\d+\.\d+$/.test(engine.version()) || !(engine.formatVersion() > 0)) throw new Error('engine name or version wrong');
if (engine.markFromName(engine.markName(1)) !== 1) throw new Error('markFromName does not invert markName');
const fixtureHex = readFileSync(new URL('../conformance/qvp1-lite.hex', import.meta.url), 'utf8').trim();
const page = engine.loadPage(Buffer.from(fixtureHex, 'hex'));
if (JSON.stringify(page.wordGroup(0)) !== '[0]' || page.wordGroup(99).length) throw new Error('wordGroup wrapper disagrees');
page.free();
console.log('ok  engine name, version, mark names and word groups round-trip');

const cjs = (await import('./index.cjs')).default;
if (!cjs.QvpEngine) throw new Error('CJS entry did not expose QvpEngine');
console.log('ok  both entry points expose QvpEngine');
