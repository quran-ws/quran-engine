/**
 * The lifetime rules of @quran.ws/qvp-react-native, the same as every other wrapper
 * (docs/API.md, conventions): a call on a freed atlas or ornament set rejects with an error that
 * names it, and a second free does nothing. The native module is the stand-in in
 * jest/qvpModule.js. Array results come from the Kotlin library, whose tests check that they
 * grow to the engine's total.
 */
import { NativeModules } from 'react-native';
import { QvpAtlas, QvpOrnaments } from '@quran.ws/qvp-react-native';

const M = NativeModules.QvpModule as any;
const callsOf = (name: string) => M.__calls.filter((c: [string, unknown[]]) => c[0] === name);

beforeEach(() => {
  M.__calls.length = 0;
  M.__answer('loadAtlas', 3);
  M.__answer('atlasPageOf', 42);
  M.__answer('loadOrnaments', 7);
  M.__answer('ornamentList', []);
  M.__answer('ornamentFind', -1);
});

test('a freed atlas rejects every call by name, and a second free does nothing', async () => {
  const atlas = await QvpAtlas.load('asset://pages/atlas.qva');
  expect(await atlas.pageOf(2, 255)).toBe(42);
  await atlas.free();
  await atlas.free();
  expect(callsOf('freeAtlas')).toHaveLength(1);
  await expect(atlas.pageOf(2, 255)).rejects.toThrow('QvpAtlas was freed');
  await expect(atlas.searchSurahs('cow')).rejects.toThrow('QvpAtlas was freed');
  expect(callsOf('atlasPageOf')).toHaveLength(1);
});

test('a freed ornament set rejects a find by name, and a second free does nothing', async () => {
  const set = await QvpOrnaments.load('asset://pages/ornaments.qvo');
  await set.free();
  await set.free();
  expect(callsOf('freeOrnaments')).toHaveLength(1);
  await expect(set.find('ayah_mark', 'mushaf-qalon')).rejects.toThrow('QvpOrnaments was freed');
  expect(callsOf('ornamentFind')).toHaveLength(0);
  expect(set.styles()).toEqual([]);
});
