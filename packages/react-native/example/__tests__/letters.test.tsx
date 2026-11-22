/**
 * The letter surface of @quran.ws/qvp-react-native: the `lettersUri` prop, the module calls
 * (attachLetters, hasLetters, letters, pathLetter, letterAtOffset, findLetters, hitTestLetter,
 * hitTestLetterView), the letter selectors, `LETTER_PART` and a style rule with `letters`. The
 * native module is the stand-in in jest/qvpModule.js, so this checks what the JavaScript side
 * sends and how it reads the answers back. The engine's answers are checked by the Android and
 * iOS packages' replays of conformance/scenarios/letters.json, and the native marshalling by
 * MarshalLettersTest in the Android module.
 */
import React from 'react';
import { NativeModules } from 'react-native';
import ReactTestRenderer from 'react-test-renderer';
import {
  LETTER_PART, Qvp, QvpPageView, Sel, T,
  type Letter, type LetterHit, type LetterMember, type QvpPageViewHandle,
} from '@quran.ws/qvp-react-native';

const M = NativeModules.QvpModule as any;
const callsOf = (name: string) => M.__calls.filter((c: [string, unknown[]]) => c[0] === name).map((c: [string, unknown[]]) => c[1]);
const last = (name: string) => callsOf(name).at(-1);

// A letter as the native module sends it: a body, a dot and a fathah, the bodies first.
const BODY: LetterMember = { path: 9, at: 2, mark: 0, markName: '', isBody: true, isCut: true };
const DOT: LetterMember = { path: 4, at: 2, mark: 16, markName: 'dot', isBody: false, isCut: false };
const FATHAH: LetterMember = { path: 5, at: 3, mark: 1, markName: 'fathah', isBody: false, isCut: false };
const LETTER: Letter = {
  word: 0, index: 1, at: 2, codePoint: 0x0628, text: 'ب', moreAt: -1, nMoreAt: 0, nMembers: 3, nMarks: 2,
  x0: 10, y0: 20, x1: 30, y1: 40, hasInk: true, members: [BODY, DOT, FATHAH], marks: [DOT, FATHAH],
};
const HIT: LetterHit = { word: 0, letter: -1, path: 7, at: 5, line: 0, distance: 0, mark: 12, markName: 'omitted_alif', isExact: true, isBody: false };

beforeEach(() => {
  M.__calls.length = 0;
  M.__answer('attachLetters', (_: number, uri: string) => uri.endsWith('.qvl'));
  M.__answer('hasLetters', true);
  M.__answer('letters', (_: number, w: number) => (w === 0 ? [LETTER] : []));
  M.__answer('pathLetter', (_: number, path: number) => (path === 9 ? { letter: 1, at: 2 } : null));
  M.__answer('letterAtOffset', (_: number, w: number, at: number) => (w === 0 && at === 2 ? 1 : null));
  M.__answer('findLetters', [{ word: 0, letter: 1 }, { word: 3, letter: 0 }]);
  M.__answer('hitTestLetter', HIT);
  M.__answer('hitTestLetterView', null);
});

test('LETTER_PART holds the QVP_LETTER_PART_* values', () => {
  expect(LETTER_PART).toEqual({ ALL: 0, BODY_AND_DOTS: 1, MARKS: 2 });
});

test('the letter selectors carry ids 16 to 19 and their arguments', () => {
  expect(Sel.letter(4, 2)).toEqual({ selector: 16, a: 4, b: 2, c: LETTER_PART.ALL });
  expect(Sel.letter(4, 2, LETTER_PART.MARKS)).toEqual({ selector: 16, a: 4, b: 2, c: 2 });
  expect(Sel.letterMark(4, 2, 1)).toEqual({ selector: 17, a: 4, b: 2, c: 1 });
  // a name goes to the native side as it is, which asks the engine; the id beside it comes from
  // the engine's table, which the stand-in module leaves empty, so it is 255 (unknown) here
  expect(Sel.letterMarkNamed(4, 2, 'shaddah')).toEqual({ selector: 18, a: 4, b: 2, c: 255, mark: 'shaddah' });
  expect(Sel.letterMarkNamed(4, 2, 12)).toEqual({ selector: 18, a: 4, b: 2, c: 12 });
  expect(Sel.wordChars(4, 1, 3)).toEqual({ selector: 19, a: 4, b: 1, c: 3 });
});

test('each letter call passes its arguments through and reads the answer back', async () => {
  expect(await Qvp.attachLetters(42, 'asset://pages/001.qvl')).toBe(true);
  expect(last('attachLetters')).toEqual([42, 'asset://pages/001.qvl']);
  expect(await Qvp.attachLetters(42, 'asset://pages/002.qvp')).toBe(false);
  expect(await Qvp.hasLetters(42)).toBe(true);
  expect(last('hasLetters')).toEqual([42]);

  const [l] = await Qvp.letters(42, 0);
  expect(last('letters')).toEqual([42, 0]);
  expect(l).toEqual(LETTER);
  // a letter's text is its character, and codePoint the same as a number
  expect([...l.text]).toHaveLength(1);
  expect(l.text.codePointAt(0)).toBe(l.codePoint);
  expect(l.marks[1]).toBe(l.members[2]);
  expect(await Qvp.letters(42, 7)).toEqual([]);

  expect(await Qvp.pathLetter(42, 9)).toEqual({ letter: 1, at: 2 });
  expect(await Qvp.pathLetter(42, 3)).toBeNull();
  expect(callsOf('pathLetter')).toEqual([[42, 9], [42, 3]]);
  expect(await Qvp.letterAtOffset(42, 0, 2)).toBe(1);
  expect(await Qvp.letterAtOffset(42, 0, 9)).toBeNull();
  expect(last('letterAtOffset')).toEqual([42, 0, 9]);

  const query = { scope: T.ayah(1, 1), text: 'ب', mark: 'fathah', part: LETTER_PART.BODY_AND_DOTS };
  expect(await Qvp.findLetters(42, query)).toEqual([{ word: 0, letter: 1 }, { word: 3, letter: 0 }]);
  expect(last('findLetters')).toEqual([42, query]);
  await Qvp.findLetters(42, { text: 0x0644, mark: 12 });
  expect(last('findLetters')).toEqual([42, { text: 0x0644, mark: 12 }]);

  expect(await Qvp.hitTestLetter(42, 12.5, 30)).toEqual(HIT);
  expect(last('hitTestLetter')).toEqual([42, 12.5, 30, null]);
  expect(await Qvp.hitTestLetterView(42, 100, 200, { maxDistance: 4, preferExact: false })).toBeNull();
  expect(last('hitTestLetterView')).toEqual([42, 100, 200, { maxDistance: 4, preferExact: false }]);
});

test('the view ref binds the letter calls to its tag', async () => {
  const ref = React.createRef<QvpPageViewHandle>();
  await ReactTestRenderer.act(() => {
    ReactTestRenderer.create(<QvpPageView ref={ref} pageUri="asset://pages/001.qvp" />);
  });
  const tag = ref.current!.tag();
  await ref.current!.attachLetters('asset://pages/001.qvl');
  await ref.current!.letters(0);
  await ref.current!.pathLetter(9);
  await ref.current!.letterAtOffset(0, 2);
  await ref.current!.findLetters({ text: 'ب' });
  await ref.current!.hitTestLetter(1, 2, { gapBias: 0.5 });
  await ref.current!.hitTestLetterView(3, 4);
  expect(await ref.current!.hasLetters()).toBe(true);
  expect(last('attachLetters')).toEqual([tag, 'asset://pages/001.qvl']);
  expect(last('letters')).toEqual([tag, 0]);
  expect(last('pathLetter')).toEqual([tag, 9]);
  expect(last('letterAtOffset')).toEqual([tag, 0, 2]);
  expect(last('findLetters')).toEqual([tag, { text: 'ب' }]);
  expect(last('hitTestLetter')).toEqual([tag, 1, 2, { gapBias: 0.5 }]);
  expect(last('hitTestLetterView')).toEqual([tag, 3, 4, null]);
  expect(last('hasLetters')).toEqual([tag]);
});

test('lettersUri and a style rule with letters reach the native view', async () => {
  let tree!: ReactTestRenderer.ReactTestRenderer;
  const letters = { scope: T.word(3), text: 'ل', part: LETTER_PART.MARKS };
  await ReactTestRenderer.act(() => {
    tree = ReactTestRenderer.create(
      <QvpPageView
        pageUri="asset://pages/001.qvp"
        wordsUri="asset://pages/001.words.json"
        lettersUri="asset://pages/001.qvl"
        styles={[
          { id: 'lam', letters, color: '#c62828', ms: 200 },
          { id: 'one', selector: Sel.letter(3, 0, LETTER_PART.BODY_AND_DOTS), color: '#1a73e8' },
        ]}
      />,
    );
  });
  const native = tree.root.find(n => (n.type as unknown) === 'QvpPageView');
  expect(native.props.lettersUri).toBe('asset://pages/001.qvl');
  expect(native.props.wordsUri).toBe('asset://pages/001.words.json');
  // the query goes as it is; the colour goes as '#rrggbbaa', as in every rule
  expect(native.props.styles).toEqual([
    { id: 'lam', letters, color: '#c62828ff', ms: 200 },
    { id: 'one', selector: { selector: 16, a: 3, b: 0, c: 1 }, color: '#1a73e8ff' },
  ]);
});
