/**
 * Forms by name. The package sends the `Form` type's camelCase names to the native module as
 * they are, and a word's `forms` come back under the same names. The native module is the
 * stand-in in jest/qvpModule.js. It answers each form with a different string, the way the
 * Kotlin side does once it reads `'rasmImlai'` as RASM_IMLAI. MarshalNamesTest in the Android
 * module checks that reading.
 */
import { NativeModules } from 'react-native';
import { Qvp } from '@quran.ws/qvp-react-native';

const M = NativeModules.QvpModule as any;
const callsOf = (name: string) => M.__calls.filter((c: [string, unknown[]]) => c[0] === name);

// The word 'ٱلْحَمْدُ' in two forms, keyed by the names the native module reads.
const FORMS: Record<string, string> = { rasmUthmani: 'ٱلْحَمْدُ', rasmImlai: 'الحمد' };

beforeEach(() => {
  M.__calls.length = 0;
  M.__answer('wordForm', (_tag: number, _index: number, form: string) => FORMS[form] ?? FORMS.rasmUthmani);
});

test('asking for rasmImlai sends that name and gets a different string than the default form', async () => {
  const fallback = await Qvp.wordForm(1, 0);
  const rasmImlai = await Qvp.wordForm(1, 0, 'rasmImlai');
  expect(callsOf('wordForm').map((c: [string, unknown[]]) => c[1][2])).toEqual(['rasmUthmani', 'rasmImlai']);
  expect(rasmImlai).toBe(FORMS.rasmImlai);
  expect(rasmImlai).not.toBe(fallback);
});

test('text, search and selectionText send the form name unchanged', async () => {
  await Qvp.text(1, 'page', { form: 'rasmImlai' });
  await Qvp.search(1, 'الحمد', { form: 'rasmImlai' });
  await Qvp.selectionText(1, 'rasmImlai');
  expect(callsOf('text')[0][1][2]).toEqual({ form: 'rasmImlai' });
  expect(callsOf('search')[0][1][2]).toEqual({ form: 'rasmImlai' });
  expect(callsOf('selectionText')[0][1][1]).toBe('rasmImlai');
});
