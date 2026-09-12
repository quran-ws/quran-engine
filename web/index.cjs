// CommonJS entry for @quran.ws/engine. See index.mjs for why this shim exists.
require('./qvp.js');

const QVP = globalThis.QVP;
if (!QVP) throw new Error('@quran.ws/engine: qvp.js did not define globalThis.QVP');

/** Absolute path of the bundled `qvp_ffi.wasm`, for fs/fetch. */
QVP.wasmPath = require('node:path').join(__dirname, 'qvp_ffi.wasm');

module.exports = QVP;
