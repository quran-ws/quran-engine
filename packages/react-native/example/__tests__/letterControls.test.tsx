/**
 * The example's letter controls (docs/EXAMPLE-APP.md, feature 12). As in smoke.test.tsx, the test
 * plays the native view's part: it sends the events the view emits and reads the `styles` prop
 * the app sends back. The native module is the stand-in in jest/qvpModule.js.
 */
import React from 'react';
import { NativeModules, Text } from 'react-native';
import ReactTestRenderer from 'react-test-renderer';
import { LETTER_PART, Sel, type Hit, type Letter, type LetterHit, type PageInfo, type Word } from '@quran.ws/qvp-react-native';
import App from '../App';

const M = NativeModules.QvpModule as any;

jest.mock('react-native-safe-area-context', () => require('react-native-safe-area-context/jest/mock').default);

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
// the third letter of the word, a ra with a shaddah and a fathah
const LETTER: Letter = {
  word: 3, index: 2, at: 2, codePoint: 0x0631, text: 'ر', moreAt: -1, nMoreAt: 0, nMembers: 3, nMarks: 2,
  x0: 20, y0: 70, x1: 30, y1: 90, hasInk: true, members: [], marks: [],
};
const ON_BODY: LetterHit = { word: 3, letter: 2, path: 22, at: 2, line: 1, distance: 0, mark: 0, markName: '', isExact: true, isBody: true };
const ON_SHADDAH: LetterHit = { ...ON_BODY, path: 23, at: 3, mark: 9, markName: 'shaddah', isBody: false };

beforeEach(() => {
  M.__calls.length = 0;
  M.__answer('surahs', []);
  M.__answer('divisions', []);
  M.__answer('ayahKeys', []);
  M.__answer('letters', [{ ...LETTER, index: 0 }, { ...LETTER, index: 1 }, LETTER]);
  M.__answer('word', WORD);
  M.__answer('hitTestLetterView', ON_BODY);
});

let mounted: ReactTestRenderer.ReactTestRenderer | null = null;
afterEach(async () => {
  const r = mounted;
  mounted = null;
  if (r) await ReactTestRenderer.act(() => r.unmount());
});

async function open(withLayer: boolean) {
  M.__answer('hasLetters', withLayer);
  let root!: ReactTestRenderer.ReactTestRenderer;
  await ReactTestRenderer.act(() => {
    root = ReactTestRenderer.create(<App />);
  });
  mounted = root;
  const view = () => root.root.find(n => (n.type as unknown) === 'QvpPageView');
  const fire = async (event: string, nativeEvent: unknown) => {
    await ReactTestRenderer.act(async () => {
      view().props[event]({ nativeEvent });
    });
  };
  // a button is the nearest node above its label that takes a press
  const button = (label: string) => {
    let n: ReactTestRenderer.ReactTestInstance | null = root.root.findAllByType(Text).find(t => t.props.children === label) ?? null;
    while (n && !n.props.onPress) n = n.parent;
    if (!n) throw new Error(`no button ${label}`);
    return n;
  };
  const press = async (label: string) => {
    await ReactTestRenderer.act(async () => {
      button(label).props.onPress();
    });
  };
  const shown = () => root.root.findAllByType(Text).map(t => [t.props.children].flat().join('')).join('\n');
  const letterRules = () => (view().props.styles as { id: string }[]).filter(r => r.id.startsWith('letter:'));
  await fire('onPageLoad', INFO);
  // a tap: the touch point first, then the word the native view found under it
  const tap = async () => {
    await fire('onTouchStart', { locationX: 120, locationY: 240 });
    await fire('onWordTap', { word: WORD, hit: HIT });
  };
  return { root, view, button, press, shown, letterRules, tap };
}

test('the view takes the page letter layer beside the page', async () => {
  const { view } = await open(true);
  expect(view().props.lettersUri).toBe('asset://pages/001.qvl');
});

test('with no layer the controls are disabled and a tap selects the word', async () => {
  const { view, button, shown, letterRules, tap } = await open(false);
  expect(shown()).toContain('no letter layer for this page');
  for (const label of ['word', 'letter', 'mark', 'Every match', 'Clear letters']) expect(button(label).props.disabled).toBe(true);
  await tap();
  expect(letterRules()).toEqual([]);
  expect(view().props.highlights).toHaveLength(1);
  expect(M.__calls.some((c: [string]) => c[0] === 'hitTestLetterView')).toBe(false);
});

test('with a layer, a letter tap adds one rule and shows the hit', async () => {
  const { root, view, press, shown, letterRules, tap } = await open(true);
  await press('letter');
  await tap();
  expect(M.__calls.filter((c: [string]) => c[0] === 'hitTestLetterView').at(-1)[1].slice(1, 3)).toEqual([120, 240]);
  expect(letterRules()).toEqual([
    { id: 'letter:1', selector: Sel.letter(3, 2, LETTER_PART.BODY_AND_DOTS), color: '#c62828ff', ms: 200, layer: expect.any(Number) },
  ]);
  // a letter tap does not select the word
  expect(view().props.highlights).toEqual([]);
  expect(shown()).toContain('ر U+0631 · 1:1:4 · letter 2 · offset 2 · 2 marks · exact');

  // every match: each ra of the page, in the next colour
  await press('Every match');
  await ReactTestRenderer.act(async () => {
    root.root.findByProps({ accessibilityLabel: 'letter colour #1a73e8' }).props.onPress();
  });
  await tap();
  expect(letterRules()[1]).toMatchObject({ letters: { scope: 'page', text: 'ر', part: LETTER_PART.BODY_AND_DOTS }, color: '#1a73e8ff' });

  await press('Clear letters');
  expect(letterRules()).toEqual([]);

  // the rules belong to the page: the next page starts with none
  await tap();
  expect(letterRules()).toHaveLength(1);
  await press('▶');
  expect(view().props.pageUri).toBe('asset://pages/002.qvp');
  expect(letterRules()).toEqual([]);
});

test('in mark mode, a mark tap colours that mark and a body tap the marks of its letter', async () => {
  const { press, shown, letterRules, tap } = await open(true);
  await press('mark');
  M.__answer('hitTestLetterView', ON_SHADDAH);
  await tap();
  expect(letterRules()[0]).toMatchObject({ selector: Sel.letterMarkNamed(3, 2, 'shaddah') });
  expect(shown()).toContain('2 marks · shaddah · exact');
  M.__answer('hitTestLetterView', ON_BODY);
  await tap();
  expect(letterRules()[1]).toMatchObject({ selector: Sel.letter(3, 2, LETTER_PART.MARKS) });

  // every match: a mark tap takes that mark on each letter, and a body tap does nothing
  await press('Every match');
  await tap();
  expect(letterRules()).toHaveLength(2);
  M.__answer('hitTestLetterView', ON_SHADDAH);
  await tap();
  expect(letterRules()[2]).toMatchObject({ letters: { scope: 'page', mark: 'shaddah', part: LETTER_PART.MARKS } });

  // a tap that finds no letter colours nothing
  M.__answer('hitTestLetterView', null);
  await tap();
  expect(letterRules()).toHaveLength(3);
});
