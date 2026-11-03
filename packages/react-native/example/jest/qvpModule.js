/**
 * A stand-in for the native QvpModule, so Jest can load @quran.ws/qvp-react-native without a
 * device. Every method records its call and resolves to the answer a test set with `answer`, or
 * to null. The engine is not here: these tests check what the JavaScript side sends and how it
 * reads the answers back.
 */
const { NativeModules } = require('react-native');
const { readFileSync } = require('fs');
const { join } = require('path');

// The defaults as the real module sends them: every QVP_DEFAULT_* in qvp.h, by camelCase name,
// as the decimal the header states. Read from the header, so no number is copied here.
function headerDefaults() {
  const header = readFileSync(join(__dirname, '..', '..', '..', '..', 'crates', 'qvp-ffi', 'include', 'qvp.h'), 'utf8');
  const out = {};
  for (const m of header.matchAll(/#define QVP_DEFAULT_([A-Z_]+)\s+(0x[0-9a-fA-F]+u?|[0-9.]+f?)/g)) {
    const name = m[1].toLowerCase().replace(/_([a-z])/g, (_, c) => c.toUpperCase());
    // `u` closes a hex colour and `f` a decimal; a hex digit f is part of the number
    out[name] = m[2].startsWith('0x') ? parseInt(m[2].replace(/u$/, ''), 16) : Number(m[2].replace(/f$/, ''));
  }
  if (!('ink' in out)) throw new Error('qvpModule mock: no QVP_DEFAULT_* found in qvp.h');
  return out;
}

const calls = [];
const answers = {};
const constants = {
  version: 'test', formatVersion: 0,
  marks: [], kinds: [], families: [], categories: [], decorations: [], divisions: [], places: [],
  defaults: headerDefaults(),
};
const module = new Proxy({}, {
  get(_, name) {
    if (name === 'getConstants') return () => constants;
    if (name === '__calls') return calls;
    if (name === '__answer') return (method, value) => { answers[method] = value; };
    if (typeof name !== 'string') return undefined;
    return (...args) => {
      calls.push([name, args]);
      const a = answers[name];
      return Promise.resolve(typeof a === 'function' ? a(...args) : a === undefined ? null : a);
    };
  },
});
NativeModules.QvpModule = module;
