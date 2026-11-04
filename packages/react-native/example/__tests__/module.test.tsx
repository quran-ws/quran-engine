/**
 * The package's module check. jest/qvpModule.js stands in for the native module; this test
 * takes it away and loads the package again, so the first error a developer sees before the
 * module is linked is the one that says so.
 */
import { NativeModules } from 'react-native';

test('a missing native module throws the message that names it, not a TypeError', () => {
  const linked = NativeModules.QvpModule;
  NativeModules.QvpModule = undefined;
  try {
    jest.isolateModules(() => {
      expect(() => require('@quran.ws/qvp-react-native')).toThrow(/QvpModule not linked/);
    });
  } finally {
    NativeModules.QvpModule = linked;
  }
});

test('the linked module loads the package', () => {
  jest.isolateModules(() => {
    expect(() => require('@quran.ws/qvp-react-native')).not.toThrow();
  });
});

test('rgba rounds alpha to the nearest step, and a NaN or out-of-range alpha clamps', () => {
  jest.isolateModules(() => {
    const { rgba } = require('@quran.ws/qvp-react-native');
    expect(rgba('#1a73e8', 0.5)).toBe('#1a73e880');
    expect(rgba('#1a73e8', NaN)).toBe('#1a73e800');
    expect(rgba('#1a73e8', -1)).toBe('#1a73e800');
    expect(rgba('#1a73e8', 2)).toBe('#1a73e8ff');
  });
});
