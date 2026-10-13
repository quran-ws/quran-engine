/**
 * A stand-in for the native QvpModule, so Jest can load @quran.ws/qvp-react-native without a
 * device. Every method records its call and resolves to the answer a test set with `answer`, or
 * to null. The engine is not here: these tests check what the JavaScript side sends and how it
 * reads the answers back.
 */
const { NativeModules } = require('react-native');

const calls = [];
const answers = {};
const constants = {
  version: 'test', formatVersion: 0,
  marks: [], kinds: [], families: [], categories: [], decorations: [], divisions: [], places: [],
  defaults: { ayahMarkScaleScan: 1.5 },
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
