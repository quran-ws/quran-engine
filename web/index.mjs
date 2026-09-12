// ESM entry for @quran.ws/engine.
//
// qvp.js is the reference wrapper and is written as an IIFE that assigns a
// single global, so that a plain <script> tag works with no build step. That
// shape cannot be imported, so this file loads it for its side effect and
// re-exports what it defined. The wrapper itself is unchanged.
import './qvp.js';

const QVP = globalThis.QVP;
if (!QVP) throw new Error('@quran.ws/engine: qvp.js did not define globalThis.QVP');

export const {
  QvpEngine, QvpPage, QvpAtlas, CanvasRenderer,
  Sel, T, KIND, FAMILY, CATEGORY, DECO, FORM, LAYER, MARK, MARKS, NONE,
  css, rgba, parseTarget,
} = QVP;

/** Absolute URL of the bundled `qvp_ffi.wasm`, for fetch/instantiate. */
export const wasmUrl = new URL('./qvp_ffi.wasm', import.meta.url);

export default QVP;
