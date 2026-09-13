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
console.log('ok  engine name, version and mark names round-trip');

const cjs = (await import('./index.cjs')).default;
if (!cjs.QvpEngine) throw new Error('CJS entry did not expose QvpEngine');
console.log('ok  both entry points expose QvpEngine');
