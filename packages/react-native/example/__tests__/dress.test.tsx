/**
 * The dress surface of @quran.ws/qvp-react-native: the ornament set (loadOrnaments, ornamentList,
 * ornamentCount, ornamentInfo, ornamentPart, ornamentFind, freeOrnaments) and the page
 * (dress, undress, dressClamp, dressView, dressInfo, dressRevision), plus the `dress` prop. The native module is the
 * stand-in in jest/qvpModule.js, so this checks what the JavaScript side sends and reads back.
 * The engine's own answers are checked by the Android SDK's tests and crates/qvp-ffi/tests/abi.rs.
 */
import React from 'react';
import { NativeModules } from 'react-native';
import ReactTestRenderer from 'react-test-renderer';
import { DEFAULTS, Qvp, QvpOrnaments, QvpPageView, type Ornament } from '@quran.ws/qvp-react-native';

const M = NativeModules.QvpModule as any;
const licence = { id: 'CC-BY-4.0', status: 'confirmed', redistributable: true, attribution: 'test' };
const part = (index: number, color: number, name: string) => ({ index, color, stroke: name === 'line', name });
const ornament = (index: number, kind: Ornament['kind'], style: string): Ornament => ({
  index, kind, lineage: 'scan', style, riwayah: "Qalun 'an Nafi'", hasSlices: kind === 'page_frame', licence,
  parts: [part(0, 0xb8860bff, 'c0'), part(1, 0x00000000, 'slot'), part(2, 0x231f20ff, 'line')],
});
const LIST = [
  ornament(0, 'ayah_mark', 'mushaf-qalon'), ornament(1, 'surah_header', 'mushaf-qalon'), ornament(2, 'page_frame', 'mushaf-qalon'),
  ornament(3, 'ayah_mark', 'font-003-regular'),
];
const INFO = { ayahMark: 0, surahHeader: 1, pageFrame: 2, ayahMarks: 7, surahHeaders: 1, frameRepeats: 12, frameStretched: false, draws: 40, revision: 3, viewBox: [-20, -20, 400, 600], overflow: [8, 8, 8, 8] };

const callsOf = (name: string) => M.__calls.filter((c: [string, unknown[]]) => c[0] === name).map((c: [string, unknown[]]) => c[1]);
const last = (name: string) => callsOf(name).at(-1);

beforeEach(() => {
  M.__calls.length = 0;
  M.__answer('loadOrnaments', 7);
  M.__answer('ornamentList', LIST);
  M.__answer('ornamentCount', LIST.length);
  M.__answer('ornamentInfo', (_: number, i: number) => LIST[i] ?? null);
  M.__answer('ornamentPart', (_: number, i: number, k: number) => LIST[i]?.parts[k] ?? null);
  M.__answer('ornamentFind', (_: number, kind: string, style: string) => LIST.findIndex(o => o.kind === kind && o.style === style));
  M.__answer('dress', (_: number, __: number, spec: any) => (spec.ayahMark === 99 ? null : INFO));
  M.__answer('dressInfo', INFO);
  M.__answer('dressRevision', INFO.revision);
  M.__answer('dressClamp', (_: number, scale: number, ox: number, oy: number) => ({ scale, offsetX: Math.min(ox, 5), offsetY: oy }));
  // the stand-in answers as the engine does for content 360 x 600 with overflow 12, 9, 12, 7
  M.__answer('dressView', (_: number, scale: number, vw: number) => {
    const [w, h] = [(360 + 12 + 12) * scale, (600 + 9 + 7) * scale];
    return { view: { scale, offsetX: Math.max((vw - w) / 2, 0) + 12 * scale, offsetY: 9 * scale }, width: w, height: h };
  });
});

test('an ornament set loads, lists, finds and groups by style', async () => {
  const set = await QvpOrnaments.load('asset://pages/ornaments.qvo');
  expect(last('loadOrnaments')).toEqual(['asset://pages/ornaments.qvo']);
  expect(last('ornamentList')).toEqual([7]);
  expect(set.id).toBe(7);
  expect(set.count).toBe(4);
  expect(set.list[2].licence.redistributable).toBe(true);
  expect(await set.find('surah_header', 'mushaf-qalon')).toBe(set.list[1]);
  expect(last('ornamentFind')).toEqual([7, 'surah_header', 'mushaf-qalon']);
  expect(await set.find('page_frame', 'font-003-regular')).toBeNull();
  expect(set.ofKind('ayah_mark').map(o => o.index)).toEqual([0, 3]);
  const styles = set.styles();
  expect(styles.map(s => s.style)).toEqual(['mushaf-qalon', 'font-003-regular']);
  expect(styles[0].pageFrame?.index).toBe(2);
  expect(styles[1].surahHeader).toBeNull();
  expect(await Qvp.ornamentCount(set.id)).toBe(4);
  expect((await Qvp.ornamentInfo(set.id, 3))?.style).toBe('font-003-regular');
  expect((await Qvp.ornamentPart(set.id, 0, 1))?.name).toBe('slot');
  await set.free();
  expect(last('freeOrnaments')).toEqual([7]);
});

test('dress sends indices, style names and colours the native side reads', async () => {
  const set = await QvpOrnaments.load('asset://pages/ornaments.qvo');
  const info = await Qvp.dress(42, set, {
    style: 'mushaf-qalon', ayahMark: set.list[3], pageFrame: null, gap: 3, ayahMarkScale: 1.2, lineArt: ['surah_header'],
    colors: [[set.list[1], 0, '#c62828'], [3, 2, 0x0a7d32ff]],
  });
  expect(info).toEqual(INFO);
  const [tag, id, spec] = last('dress');
  expect([tag, id]).toEqual([42, 7]);
  // a kind key overrides the style; a kind left out takes the style's name; null is none
  expect(spec).toMatchObject({ ayahMark: 3, surahHeader: 'mushaf-qalon', pageFrame: null, gap: 3, ayahMarkScale: 1.2, lineArt: ['surah_header'] });
  expect(spec.colors).toEqual([[1, 0, '#c62828ff'], [3, 2, '#0a7d32ff']]);
  expect((await Qvp.dress(42, 7, { lineArt: true })) && last('dress')[2].lineArt).toEqual(['ayah_mark', 'surah_header', 'page_frame']);
  expect(last('dress')[2]).toMatchObject({ ayahMark: null, surahHeader: null, pageFrame: null });
  // the engine refused: null comes back
  expect(await Qvp.dress(42, set, { ayahMark: 99 })).toBeNull();
  expect(await Qvp.dressInfo(42)).toEqual(INFO);
  expect(await Qvp.dressRevision(42)).toBe(3);
  // the engine clamps the view over the content plus the overflow; the wrapper only passes it through
  expect(await Qvp.dressClamp(42, { scale: 2, offsetX: 40, offsetY: -9 }, 390, 700)).toEqual({ scale: 2, offsetX: 5, offsetY: -9 });
  expect(last('dressClamp')).toEqual([42, 2, 40, -9, 390, 700]);
  // a host that scrolls the page: the size is content plus overflow, the border's top at 0
  const shown = await Qvp.dressView(42, 1, 390);
  expect(last('dressView')).toEqual([42, 1, 390]);
  expect([shown.width, shown.height]).toEqual([360 + 12 + 12, 600 + 9 + 7]);
  expect(shown.view.offsetY - shown.view.scale * 9).toBe(0);
  await Qvp.undress(42);
  expect(last('undress')).toEqual([42]);
});

test('the dress prop goes to the native view with the set id and a normalised spec', async () => {
  const set = await QvpOrnaments.load('asset://pages/ornaments.qvo');
  let tree: ReactTestRenderer.ReactTestRenderer;
  await ReactTestRenderer.act(() => {
    tree = ReactTestRenderer.create(<QvpPageView pageUri="asset://pages/604.qvp" dress={{ ornaments: set, style: 'mushaf-qalon', colors: [[0, 0, '#fff']] }} />);
  });
  const native = tree!.root.findAll(n => typeof n.type === 'string' && n.props.dress !== undefined)[0];
  expect(native.props.dress).toMatchObject({ ornaments: 7, ayahMark: 'mushaf-qalon', surahHeader: 'mushaf-qalon', pageFrame: 'mushaf-qalon', colors: [[0, 0, '#ffffffff']] });
  expect(DEFAULTS.AYAH_MARK_SCALE_SCAN).toBe(1.5);
});
