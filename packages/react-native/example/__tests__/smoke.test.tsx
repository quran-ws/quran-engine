/**
 * The example's UI smoke test (docs/standards/TESTING.md): open a page, tap a word, highlight.
 * Jest has no device, so the test plays the native view's part. It sends the page-load and
 * word-tap events the view emits, and reads the `highlights` prop the app sends back to the
 * view. The native module is the stand-in in jest/qvpModule.js. The highlight the engine
 * draws from that prop is checked by the Android SDK's tests.
 */
import React from 'react';
import { NativeModules, Text } from 'react-native';
import ReactTestRenderer from 'react-test-renderer';
import { LAYER, T, type Hit, type PageInfo, type Word } from '@quran.ws/qvp-react-native';
import App from '../App';

const M = NativeModules.QvpModule as any;

// The real provider waits for the device's insets before it renders anything. The package's
// mock gives fixed insets at once.
jest.mock('react-native-safe-area-context', () => require('react-native-safe-area-context/jest/mock').default);

// Page 1 and its fourth word, as the native view reports them.
const INFO: PageInfo = {
  page: 1, width: 345, height: 550, nLines: 8, nAyahs: 7, nWords: 33, nPaths: 200, nDecorations: 9,
  lineSpacing: 60, forms: ['rasmUthmani'], loadMs: 1, bytes: 30000,
};
const WORD: Word = {
  index: 3, surah: 1, ayah: 1, word: 4, line: 2, lineIndex: 1, ayahIndex: 0,
  x0: 10, y0: 60, x1: 60, y1: 100, text: 'ٱلرَّحِيمِ', firstPath: 20, nPaths: 6,
  wordKey: '1:1:4', ayahKey: '1:1', forms: {}, label: '', paths: [],
};
const HIT: Hit = { word: 3, path: 21, decoration: -1, line: 1, distance: 0, isExact: true, wordKey: '1:1:4', ayahKey: '1:1' };

beforeEach(() => {
  M.__calls.length = 0;
  // the page queries onPageLoad makes; the page has no surah start or division to report
  M.__answer('surahs', []);
  M.__answer('divisions', []);
  M.__answer('ayahKeys', []);
});

test('open page 1, tap a word, and the app highlights it', async () => {
  let root!: ReactTestRenderer.ReactTestRenderer;
  await ReactTestRenderer.act(() => {
    root = ReactTestRenderer.create(<App />);
  });
  const view = () => root.root.find(n => (n.type as unknown) === 'QvpPageView');
  const fire = async (event: string, nativeEvent: unknown) => {
    await ReactTestRenderer.act(async () => {
      view().props[event]({ nativeEvent });
    });
  };
  const shown = () => root.root.findAllByType(Text).map(t => [t.props.children].flat().join('')).join('\n');

  // The app opens the first bundled page, and the view reports it loaded.
  expect(view().props.pageUri).toBe('asset://pages/001.qvp');
  await fire('onPageLoad', INFO);
  expect(view().props.highlights).toEqual([]);

  // A tap on a word: the app sends one highlight on that word, in the selection layer.
  await fire('onWordTap', { word: WORD, hit: HIT });
  const highlights = view().props.highlights;
  expect(highlights).toHaveLength(1);
  expect(highlights[0]).toMatchObject({ id: 'sel', target: T.word(WORD.index), style: { layer: LAYER.SELECTION } });
  expect(highlights[0].style.ink).toBeTruthy();
  // The selection panel shows the word and its key.
  expect(shown()).toContain(WORD.text);
  expect(shown()).toContain(`wordKey ${WORD.wordKey}`);

  // A second tap on the same word clears the highlight, as on every platform.
  await fire('onWordTap', { word: WORD, hit: HIT });
  expect(view().props.highlights).toEqual([]);

  // A tap on empty paper clears it too.
  await fire('onWordTap', { word: WORD, hit: HIT });
  expect(view().props.highlights).toHaveLength(1);
  await fire('onEmptyTap', {});
  expect(view().props.highlights).toEqual([]);

  await ReactTestRenderer.act(() => root.unmount());
});
