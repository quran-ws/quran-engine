// Smoke test: every public entry point loads and the engine answers.
// Drawing is not exercised — buildPaths() and drawBoxes() need a DOM Path2D.
import { readFileSync } from 'node:fs';
import { QvpEngine, wasmUrl } from './index.mjs';

const engine = await QvpEngine.init(readFileSync(wasmUrl));
const strip = engine.strip('الْحَمْدُ');
if (!strip) throw new Error('engine.strip() returned nothing');

console.log('ok  wasm instantiated, engine.strip() ->', strip);

const cjs = (await import('./index.cjs')).default;
if (!cjs.QvpEngine) throw new Error('CJS entry did not expose QvpEngine');
console.log('ok  both entry points expose QvpEngine');
