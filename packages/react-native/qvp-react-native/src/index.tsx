/**
 * @quran.ws/qvp-react-native — typed JS API over the native module.
 *
 * Names follow docs/API.md. Nothing here decides anything: targets, selectors and colours are marshalled
 * to the Kotlin library (`ws.quran.qvp`), which is JNI over crates/qvp-ffi/include/qvp.h.
 * Declarative props (`highlights`, `styles`, `theme`, `mask`, `reveal`) are reconciled against engine
 * handles on the native side, so JS never sees a handle.
 */
import React, { forwardRef, useImperativeHandle, useMemo, useRef } from 'react';
import {
  findNodeHandle,
  NativeModules,
  requireNativeComponent,
  type HostComponent,
  type NativeSyntheticEvent,
  type ViewProps,
} from 'react-native';

// ── constants (mirror qvp.h) ─────────────────────────────────────────────────────────────────
export const KIND = { BODY: 0, MARK: 1, AYAH_NUMBER: 2, AYAH_MARK_ORNAMENT: 3, HEADER_INK: 4, OTHER: 255 } as const;
export const FAMILY = { NONE: 0, DIACRITIC: 1, TANWIN: 2, DOTS: 3, WAQF: 4, SIFR: 5, SAJDAH: 6, READING_SIGN: 7 } as const;
export const CATEGORY = { NONE: 0, HARAKAH: 1, TANWIN: 2, LETTER_DOT: 3, ORTHOGRAPHIC: 4, DABT: 5, WAQF: 6, READING_SIGN: 7, STANDALONE: 8 } as const;
export const DECO = { AYAH_MARK: 0, SURAH_NAME: 1, BASMALAH: 2, DIVISION_MARK: 3, SAJDAH_MARK: 4 } as const;
export const LAYER = { BASE: 0, THEME: 10, HIGHLIGHT: 50, SELECTION: 60, TOP: 100 } as const;
export const DIVISION = { JUZ: 0, HIZB: 1, NISF: 2, RUBU_AL_HIZB: 3 } as const;
export const MARKS = ['', 'fathah', 'kasrah', 'dammah', 'tanwin_al_fath', 'tanwin_al_kasr', 'tanwin_al_damm', 'shaddah', 'sukun', 'maddah', 'hamzah', 'hamzat_al_wasl', 'omitted_alif', 'small_waw', 'small_yaa', 'small_noon', 'dot', 'two_dots', 'three_dots', 'rounded_zero', 'rectangular_zero', 'waqf_jaiz_mustawi_al_tarafayn', 'waqf_jaiz_waqf_awla', 'waqf_jaiz_wasl_awla', 'waqf_lazim', 'waqf_al_muanaqah', 'saktah', 'small_meem', 'hizb', 'sajdah', 'sajdah_mark', 'sajdah_line', 'seen_al_qiraah', 'tashil', 'ishmam', 'imalah'];
const KIND_NAMES = ['body', 'mark', 'ayah-number', 'ayah-ornament', 'header-ink'];
const FAMILY_NAMES = ['none', 'diacritic', 'tanwin', 'dots', 'waqf', 'sifr', 'sajdah', 'reading_sign'];
const CATEGORY_NAMES = ['none', 'harakah', 'tanwin', 'letter_dot', 'orthographic', 'dabt', 'waqf', 'reading_sign', 'standalone'];
const DECO_NAMES = ['ayah-mark', 'surah-name', 'basmalah', 'division-mark', 'sajdah-mark', 'page-number', 'running-head'];
export function markId(name: string | number): number {
  if (typeof name === 'number') return name;
  const i = MARKS.indexOf(name);
  return i < 0 ? 255 : i;
}
const named = (v: string | number, names: string[]) => (typeof v === 'number' ? v : Math.max(0, names.indexOf(v)));

// ── colours ──────────────────────────────────────────────────────────────────────────────────
export type Color = string | number;
/** Any accepted colour ('#rgb' | '#rrggbb' | '#rrggbbaa' | 0xRRGGBBAA) → '#rrggbbaa'. */
export function css(c: Color | null | undefined): string | undefined {
  if (c == null) return undefined;
  if (typeof c === 'number') return '#' + (c >>> 0).toString(16).padStart(8, '0');
  let h = c.trim().replace(/^#/, '');
  if (h.length === 3) h = h.split('').map(x => x + x).join('');
  if (h.length === 6) h += 'ff';
  return '#' + h.toLowerCase();
}
/** Colour with a replaced alpha (0..1). */
export function rgba(c: Color, alpha: number): string {
  const h = css(c)!;
  return h.slice(0, 7) + Math.round(Math.max(0, Math.min(1, alpha)) * 255).toString(16).padStart(2, '0');
}

// ── targets and selectors (same shapes as web/qvp.js T / Sel) ────────────────────────────────
export interface TargetObj { kind: number; a?: number; b?: number; c?: number; words?: number[] }
/** 'page' | '2:255' | '2:255:3' | '2:255-257' | 'line:7' | 'surah:2' | word index | [indices] | T.* */
export type Target = string | number | number[] | TargetObj;
export const T = {
  page: (): TargetObj => ({ kind: 0 }),
  word: (i: number): TargetObj => ({ kind: 1, a: i }),
  words: (ws: number[]): TargetObj => ({ kind: 2, words: ws }),
  ayah: (s: number, a: number): TargetObj => ({ kind: 3, a: s, b: a }),
  ayahRange: (s: number, a: number, b: number): TargetObj => ({ kind: 4, a: s, b: a, c: b }),
  line: (n: number): TargetObj => ({ kind: 5, a: n }),
  surah: (s: number): TargetObj => ({ kind: 6, a: s }),
  range: (a: number, b: number): TargetObj => ({ kind: 7, a, b }),
};
export interface Selector { kind: number; a?: number; b?: number; c?: number; mark?: string }
export const Sel = {
  page: (): Selector => ({ kind: 0 }),
  path: (p: number): Selector => ({ kind: 1, a: p }),
  wordPath: (w: number, nth: number): Selector => ({ kind: 2, a: w, b: nth }),
  /** nth mark of the word (0-based, marks only) */
  wordMark: (w: number, nth: number): Selector => ({ kind: 3, a: w, b: nth }),
  wordMarkNamed: (w: number, mark: string, nth = 0): Selector => ({ kind: 4, a: w, b: markId(mark), c: nth, mark }),
  wordBody: (w: number): Selector => ({ kind: 5, a: w }),
  wordMarks: (w: number): Selector => ({ kind: 6, a: w }),
  word: (w: number): Selector => ({ kind: 7, a: w }),
  ayah: (s: number, a: number): Selector => ({ kind: 8, a: s, b: a }),
  line: (n: number): Selector => ({ kind: 9, a: n }),
  mark: (m: string | number): Selector => ({ kind: 10, a: markId(m) }),
  category: (c: string | number): Selector => ({ kind: 11, a: named(c, CATEGORY_NAMES) }),
  family: (f: string | number): Selector => ({ kind: 12, a: named(f, FAMILY_NAMES) }),
  kind: (k: string | number): Selector => ({ kind: 13, a: named(k, KIND_NAMES) }),
  deco: (k: string | number): Selector => ({ kind: 14, a: named(k, DECO_NAMES) }),
  decoIdx: (d: number): Selector => ({ kind: 15, a: d }),
};

// ── records ──────────────────────────────────────────────────────────────────────────────────
export type Form = 'rasmUthmani' | 'rasmImlai' | 'qpc' | 'rasm' | 'search';
export type HighlightMode = 'ink' | 'band' | 'both';
export type MaskMode = 'hide' | 'block' | 'blur';
export type DivisionKind = 'juz' | 'hizb' | 'nisf' | 'rubuAlHizb';
export interface WordPath { idx: number; kind: number; kindName: string; mark: number; markName: string; nthMark: number; nthInWord: number; category: number; categoryName: string; family: number; familyName: string; line: number }
export interface Word {
  idx: number; surah: number; ayah: number; word: number; line: number; lineIdx: number; ayahIdx: number;
  x0: number; y0: number; x1: number; y1: number; text: string; firstPath: number; nPaths: number;
  wordKey: string; ayahKey: string; forms: Partial<Record<Form, string>>; label: string; paths: WordPath[];
}
export interface Deco { idx: number; kind: number; kindName: string; surah: number; ayah: number; line: number; x0: number; y0: number; x1: number; y1: number; text: string; firstPath: number; nPaths: number }
export interface Hit { word: number; path: number; deco: number; line: number; distance: number; exact: boolean; wordKey: string | null; ayahKey: string | null }
export interface Ayah { idx: number; surah: number; ayah: number; fragment: number; fragments: number; flags: number; rubuAlHizb: number; firstWord: number; nWords: number; ayahMarkDeco: number; bbox: number[] }
export interface Line { idx: number; lineNo: number; isHeader: boolean; firstWord: number; nWords: number; bbox: number[]; bandY0: number; bandY1: number; centre: number }
export interface SelectionInfo { words: number[]; text: string; citation: string; textWithCitation: string }
export interface PageInfo { page: number; width: number; height: number; nLines: number; nAyahs: number; nWords: number; nPaths: number; nDecos: number; naturalPitch: number; forms: Form[]; loadMs: number; bytes: number; uri?: string }
export interface Match { word: number; index: number; loose: boolean; wordKey: string; text: string }
export interface Surah { number: number; ayahCount: number; hasBanner: boolean; hasBasmalah: boolean; place: string; bannerDeco: number; arabic: string; latin: string; english: string }
export interface Division { kind: DivisionKind; n: number; surah: number; ayah: number; line: number; ayahIdx: number }
export interface Marker { deco: number; surah: number; ayah: number; line: number; cx: number; cy: number; r: number; ornamentPath: number; numeralPath: number }
export interface Rosette { deco: number; surah: number; ayah: number; juz: number; hizb: number; nisf: number; rubuAlHizb: number; rubuAlHizbInHizb: number }
export interface Sajdah { deco: number; surah: number; ayah: number; signPath: number }
export interface CropBox { x0: number; y0: number; x1: number; y1: number; nWords: number; ayahMarkDeco: number }
export interface Layout { scale: number; ox: number; oy: number; contentW: number; contentH: number; pitch: number; lineDy: number[]; slots: number[][] }
export interface Stats { loadMs: number; bytes: number; baseMs: number; overlayMs: number; basePaths: number; overlayPaths: number; bands: number; hitUs: number; animating: boolean; styleHandles: number; highlightHandles: number; engineVersion: number; viewScale: number; layout: Layout | null }
export interface AtlasSurah { n: number; number: number; page: number; ayahCount: number; place: string; arabic: string; latin: string; english: string }
export interface AtlasRubuAlHizb { rubuAlHizb: number; surah: number; ayah: number; page: number; ayahKey: string }

export interface HighlightStyle { mode?: HighlightMode; ink?: Color; band?: Color; height?: 'pitch' | 'ink'; padX?: number; padY?: number; radius?: number; seam?: number; ms?: number; layer?: number }
/** One declarative highlight; the native side keeps id → handle and calls highlight / rehighlight / restyleHighlight / unhighlight on diff. */
export interface Highlight { id: string; target: Target; style?: HighlightStyle }
/** One declarative style rule: `selector` (→ page.style) or `target` (→ page.styleTarget); `hide` → page.hide(selector). */
export interface StyleRule { id: string; selector?: Selector; target?: Target; color?: Color; ms?: number; layer?: number; hide?: boolean }
export interface Theme { ink?: Color; diacritics?: Color; dots?: Color; waqf?: Color; sifr?: Color; ayahMark?: Color; numeral?: Color; headers?: Color; marks?: Record<string, Color>; ms?: number }
export interface Mask { target: Target; mode?: MaskMode; blockColor?: Color; padX?: number; padY?: number; radius?: number; reverse?: boolean }
/** Greyed-page reveal: `revealStart(lit, byAyah, grey, ink, ayahMarks, ms)` then `revealGoto(at)` whenever `at` changes; null → revealStop. */
export interface Reveal { lit?: number; byAyah?: boolean; grey?: Color; ink?: Color; ayahMarks?: boolean; ms?: number; at?: number }

// ── the view ─────────────────────────────────────────────────────────────────────────────────
export interface QvpPageViewProps extends ViewProps {
  /** 'asset://pages/042.qvp' | 'file:///…' | '/abs/path' | 'base64:…' — the package ships no page data. */
  pageUri?: string;
  pageBase64?: string;
  /** optional NNN.words.json sidecar (derived forms), same URI schemes */
  wordsUri?: string;
  padTop?: number; padBottom?: number; padSide?: number;
  lineSpacing?: number; lineGap?: number; fillHeight?: boolean;
  paperColor?: Color; defaultInk?: Color; selectionBand?: Color;
  selectionEnabled?: boolean; zoomEnabled?: boolean; hitMaxDistance?: number;
  theme?: Theme | null;
  styles?: StyleRule[];
  highlights?: Highlight[];
  mask?: Mask | null;
  reveal?: Reveal | null;
  onWordTap?: (e: { word: Word; hit: Hit }) => void;
  onDecoTap?: (e: { deco: Deco; hit: Hit }) => void;
  onEmptyTap?: () => void;
  onSelectionChanged?: (e: SelectionInfo) => void;
  onPageLoad?: (e: PageInfo) => void;
  onRevealChanged?: (e: { steps: number; at: number | null }) => void;
  onError?: (e: { message: string }) => void;
}
type NativeProps = ViewProps & Omit<QvpPageViewProps, 'onWordTap' | 'onDecoTap' | 'onEmptyTap' | 'onSelectionChanged' | 'onPageLoad' | 'onRevealChanged' | 'onError'> & {
  onWordTap?: (e: NativeSyntheticEvent<{ word: Word; hit: Hit }>) => void;
  onDecoTap?: (e: NativeSyntheticEvent<{ deco: Deco; hit: Hit }>) => void;
  onEmptyTap?: (e: NativeSyntheticEvent<{}>) => void;
  onSelectionChanged?: (e: NativeSyntheticEvent<SelectionInfo>) => void;
  onPageLoad?: (e: NativeSyntheticEvent<PageInfo>) => void;
  onRevealChanged?: (e: NativeSyntheticEvent<{ steps: number; at: number | null }>) => void;
  onError?: (e: NativeSyntheticEvent<{ message: string }>) => void;
};
const NativeQvpPageView = requireNativeComponent<NativeProps>('QvpPageView') as HostComponent<NativeProps>;

const colorKeys = ['ink', 'band', 'diacritics', 'dots', 'waqf', 'sifr', 'ayahMark', 'numeral', 'headers', 'color', 'blockColor', 'grey'];
function normColors<X>(o: X): X {
  if (o == null || typeof o !== 'object') return o;
  if (Array.isArray(o)) return o.map(normColors) as X;
  const out: any = {};
  for (const [k, v] of Object.entries(o as any)) {
    if (colorKeys.includes(k) && (typeof v === 'string' || typeof v === 'number')) out[k] = css(v);
    else if (k === 'marks' && v && typeof v === 'object') out[k] = Object.fromEntries(Object.entries(v as any).map(([m, c]) => [m, css(c as Color)]));
    else if (k === 'style' || k === 'target' || k === 'selector') out[k] = k === 'style' ? normColors(v) : v;
    else out[k] = v;
  }
  return out;
}

/** Imperative handle: the page-bound module API plus the react tag. */
export type QvpPageViewHandle = PageApi & { tag: () => number };

export const QvpPageView = forwardRef<QvpPageViewHandle, QvpPageViewProps>(function QvpPageView(props, ref) {
  const native = useRef<any>(null);
  const { theme, styles, highlights, mask, reveal, paperColor, defaultInk, selectionBand, onWordTap, onDecoTap, onEmptyTap, onSelectionChanged, onPageLoad, onRevealChanged, onError, ...rest } = props;
  const tag = () => findNodeHandle(native.current) ?? -1;
  useImperativeHandle(ref, () => ({ tag, ...bindPage(tag) }), []);
  const nTheme = useMemo(() => (theme ? normColors(theme) : null), [theme]);
  const nStyles = useMemo(() => (styles ? normColors(styles) : []), [styles]);
  const nHighlights = useMemo(() => (highlights ? normColors(highlights) : []), [highlights]);
  const nMask = useMemo(() => (mask ? normColors(mask) : null), [mask]);
  const nReveal = useMemo(() => (reveal ? normColors(reveal) : null), [reveal]);
  return (
    <NativeQvpPageView
      ref={native}
      {...rest}
      paperColor={css(paperColor)}
      defaultInk={css(defaultInk)}
      selectionBand={css(selectionBand)}
      theme={nTheme}
      styles={nStyles}
      highlights={nHighlights}
      mask={nMask}
      reveal={nReveal}
      onWordTap={onWordTap ? e => onWordTap(e.nativeEvent) : undefined}
      onDecoTap={onDecoTap ? e => onDecoTap(e.nativeEvent) : undefined}
      onEmptyTap={onEmptyTap ? () => onEmptyTap() : undefined}
      onSelectionChanged={onSelectionChanged ? e => onSelectionChanged(e.nativeEvent) : undefined}
      onPageLoad={onPageLoad ? e => onPageLoad(e.nativeEvent) : undefined}
      onRevealChanged={onRevealChanged ? e => onRevealChanged(e.nativeEvent) : undefined}
      onError={onError ? e => onError(e.nativeEvent) : undefined}
    />
  );
});

// ── the module ───────────────────────────────────────────────────────────────────────────────
const M = NativeModules.QvpModule as any;
if (!M) throw new Error('@quran.ws/qvp-react-native: native module QvpModule not linked (Android only for now; see README)');

export interface SearchOptions { form?: Form; mode?: 'includes' | 'exact' | 'prefix'; normalize?: boolean; loose?: boolean; limit?: number }
export interface TextOptions { form?: Form; wordSep?: string; lineSep?: string }
export interface CropOptions { pad?: number; keepAyahMarks?: boolean; background?: Color }
export interface HitOptions { maxDistance?: number; gapBias?: number; exactFirst?: boolean }

/** Page-bound queries; `tag` is the react tag of a mounted `<QvpPageView />` (`findNodeHandle`). */
export const Qvp = {
  info: (tag: number): Promise<PageInfo> => M.info(tag),
  words: (tag: number): Promise<Word[]> => M.words(tag),
  word: (tag: number, idx: number): Promise<Word | null> => M.word(tag, idx),
  ayahs: (tag: number): Promise<Ayah[]> => M.ayahs(tag),
  lines: (tag: number): Promise<Line[]> => M.lines(tag),
  decos: (tag: number): Promise<Deco[]> => M.decos(tag),
  findWord: (tag: number, s: number, a: number, w: number): Promise<number> => M.findWord(tag, s, a, w),
  resolve: (tag: number, target: Target): Promise<number[]> => M.resolve(tag, target),
  wordForm: (tag: number, idx: number, form: Form = 'rasmUthmani'): Promise<string> => M.wordForm(tag, idx, form),
  hasForm: (tag: number, form: Form): Promise<boolean> => M.hasForm(tag, form),
  attachWords: (tag: number, json: string | object): Promise<number> => M.attachWords(tag, typeof json === 'string' ? json : JSON.stringify(json)),
  surahs: (tag: number): Promise<Surah[]> => M.surahs(tag),
  divisions: (tag: number): Promise<Division[]> => M.divisions(tag),
  ayahMarks: (tag: number): Promise<Marker[]> => M.ayahMarks(tag),
  rosettes: (tag: number): Promise<Rosette[]> => M.rosettes(tag),
  sajdahs: (tag: number): Promise<Sajdah[]> => M.sajdahs(tag),
  ayahKeys: (tag: number): Promise<{ surah: number; ayah: number; ayahKey: string }[]> => M.ayahKeys(tag),
  ayahWordCount: (tag: number, s: number, a: number): Promise<{ count: number; complete: boolean }> => M.ayahWordCount(tag, s, a),
  reciteMap: (tag: number, s: number, a: number, nSegments: number): Promise<number[] | null> => M.reciteMap(tag, s, a, nSegments),
  wordLabel: (tag: number, i: number): Promise<string> => M.wordLabel(tag, i),
  ayahLabel: (tag: number, i: number): Promise<string> => M.ayahLabel(tag, i),
  text: (tag: number, target: Target = 'page', opts?: TextOptions): Promise<string> => M.text(tag, target, opts ?? null),
  search: (tag: number, query: string, opts?: SearchOptions): Promise<Match[]> => M.search(tag, query, opts ?? null),
  citation: (tag: number, words: number[]): Promise<string> => M.citation(tag, words),
  hitTestViewEx: (tag: number, x: number, y: number, opts?: HitOptions): Promise<Hit | null> => M.hitTestViewEx(tag, x, y, opts ?? null),
  wordBoxView: (tag: number, i: number): Promise<{ x0: number; y0: number; x1: number; y1: number } | null> => M.wordBoxView(tag, i),
  currentLayout: (tag: number): Promise<Layout | null> => M.currentLayout(tag),
  relayout: (tag: number): Promise<void> => M.relayout(tag),
  resetView: (tag: number): Promise<void> => M.resetView(tag),
  stats: (tag: number): Promise<Stats | null> => M.stats(tag),
  select: (tag: number, anchor: number, focus: number = anchor): Promise<SelectionInfo> => M.select(tag, anchor, focus),
  clearSelection: (tag: number): Promise<void> => M.clearSelection(tag),
  selection: (tag: number): Promise<SelectionInfo> => M.selection(tag),
  selectionText: (tag: number, form: Form = 'rasmUthmani', citation = false): Promise<string> => M.selectionText(tag, form, citation),
  revealNext: (tag: number, n = 1): Promise<number> => M.revealNext(tag, n),
  hideBack: (tag: number, n = 1): Promise<number> => M.hideBack(tag, n),
  revealWord: (tag: number, i: number): Promise<boolean> => M.revealWord(tag, i),
  hideWord: (tag: number, i: number): Promise<boolean> => M.hideWord(tag, i),
  revealAll: (tag: number): Promise<void> => M.revealAll(tag),
  hideAll: (tag: number): Promise<void> => M.hideAll(tag),
  maskHidden: (tag: number): Promise<number[]> => M.maskHidden(tag),
  maskWords: (tag: number): Promise<number[]> => M.maskWords(tag),
  revealSteps: (tag: number): Promise<number> => M.revealSteps(tag),
  revealAt: (tag: number): Promise<number | null> => M.revealAt(tag),
  revealStepOf: (tag: number, i: number): Promise<number> => M.revealStepOf(tag, i),
  cropBox: (tag: number, target: Target, opts?: CropOptions): Promise<CropBox | null> => M.cropBox(tag, target, opts ?? null),
  cropSvg: (tag: number, target: Target, opts?: CropOptions): Promise<string | null> => M.cropSvg(tag, target, opts ? { ...opts, background: css(opts.background) } : null),

  // engine-wide
  strip: (s: string): Promise<string> => M.arabic('strip', s),
  fold: (s: string): Promise<string> => M.arabic('fold', s),
  normalize: (s: string): Promise<string> => M.arabic('normalize', s),
  looseKey: (s: string): Promise<string> => M.arabic('loose', s),
  arabic: (kind: 'strip' | 'fold' | 'normalize' | 'loose', s: string): Promise<string> => M.arabic(kind, s),
  gapToFill: (pageW: number, pageH: number, lines: number, viewW: number, viewH: number, max = 0): Promise<number> => M.gapToFill(pageW, pageH, lines, viewW, viewH, max),
  wastedFraction: (pageW: number, pageH: number, viewW: number, viewH: number): Promise<number> => M.wastedFraction(pageW, pageH, viewW, viewH),
  markName: (m: number): string => MARKS[m] ?? 'unknown',
  kindName: (k: number): string => KIND_NAMES[k] ?? 'other',
  categoryName: (c: number): string => CATEGORY_NAMES[c] ?? '',
  familyName: (f: number): string => FAMILY_NAMES[f] ?? '',
  version: (M.getConstants?.().version ?? 0) as number,

  // atlas
  loadAtlas: (uri: string): Promise<number> => M.loadAtlas(uri),
  freeAtlas: (id: number): Promise<void> => M.freeAtlas(id),
  atlasPageOf: (id: number, s: number, a: number): Promise<number | null> => M.atlasPageOf(id, s, a),
  atlasPageRange: (id: number, page: number): Promise<{ first: { surah: number; ayah: number }; last: { surah: number; ayah: number } } | null> => M.atlasPageRange(id, page),
  atlasPages: (id: number): Promise<number> => M.atlasPages(id),
  atlasSurah: (id: number, n: number): Promise<AtlasSurah | null> => M.atlasSurah(id, n),
  atlasSurahs: (id: number): Promise<AtlasSurah[]> => M.atlasSurahs(id),
  atlasPageOfSurah: (id: number, n: number): Promise<number | null> => M.atlasPageOfSurah(id, n),
  atlasDivision: (id: number, kind: DivisionKind, n: number): Promise<AtlasRubuAlHizb | null> => M.atlasDivision(id, kind, n),
  atlasDivisionAt: (id: number, kind: DivisionKind, s: number, a: number): Promise<number | null> => M.atlasDivisionAt(id, kind, s, a),
  atlasJuzAt: (id: number, s: number, a: number): Promise<number | null> => M.atlasJuzAt(id, s, a),
  atlasPagesOfJuz: (id: number, n: number): Promise<[number, number] | null> => M.atlasPagesOfJuz(id, n),
  atlasFindSurah: (id: number, text: string): Promise<AtlasSurah[]> => M.atlasFindSurah(id, text),
};

/** Cross-page lookup (atlas.qva). `const atlas = await QvpAtlas.load('asset://pages/atlas.qva')`. */
export class QvpAtlas {
  private constructor(public readonly id: number) {}
  static async load(uri: string): Promise<QvpAtlas> { return new QvpAtlas(await Qvp.loadAtlas(uri)); }
  free() { return Qvp.freeAtlas(this.id); }
  pageOf(s: number, a: number) { return Qvp.atlasPageOf(this.id, s, a); }
  pageRange(page: number) { return Qvp.atlasPageRange(this.id, page); }
  pages() { return Qvp.atlasPages(this.id); }
  surah(n: number) { return Qvp.atlasSurah(this.id, n); }
  surahs() { return Qvp.atlasSurahs(this.id); }
  pageOfSurah(n: number) { return Qvp.atlasPageOfSurah(this.id, n); }
  division(kind: DivisionKind, n: number) { return Qvp.atlasDivision(this.id, kind, n); }
  juz(n: number) { return Qvp.atlasDivision(this.id, 'juz', n); }
  hizb(n: number) { return Qvp.atlasDivision(this.id, 'hizb', n); }
  rubuAlHizb(n: number) { return Qvp.atlasDivision(this.id, 'rubuAlHizb', n); }
  divisionAt(kind: DivisionKind, s: number, a: number) { return Qvp.atlasDivisionAt(this.id, kind, s, a); }
  juzAt(s: number, a: number) { return Qvp.atlasJuzAt(this.id, s, a); }
  pagesOfJuz(n: number) { return Qvp.atlasPagesOfJuz(this.id, n); }
  findSurah(text: string) { return Qvp.atlasFindSurah(this.id, text); }
}

type Tail<F> = F extends (tag: number, ...rest: infer R) => infer Ret ? (...rest: R) => Ret : never;
const pageMethods = ['info', 'words', 'word', 'ayahs', 'lines', 'decos', 'findWord', 'resolve', 'wordForm', 'hasForm', 'attachWords', 'surahs', 'divisions', 'ayahMarks', 'rosettes', 'sajdahs', 'ayahKeys', 'ayahWordCount', 'reciteMap', 'wordLabel', 'ayahLabel', 'text', 'search', 'citation', 'hitTestViewEx', 'wordBoxView', 'currentLayout', 'relayout', 'resetView', 'stats', 'select', 'clearSelection', 'selection', 'selectionText', 'revealNext', 'hideBack', 'revealWord', 'hideWord', 'revealAll', 'hideAll', 'maskHidden', 'maskWords', 'revealSteps', 'revealAt', 'revealStepOf', 'cropBox', 'cropSvg'] as const;
type PageMethod = (typeof pageMethods)[number];
export type PageApi = { [K in PageMethod]: Tail<(typeof Qvp)[K]> };
function bindPage(tag: () => number): PageApi {
  const out: any = {};
  for (const k of pageMethods) out[k] = (...args: any[]) => (Qvp[k] as any)(tag(), ...args);
  return out;
}

/**
 * `const qvp = useQvp(); <QvpPageView ref={qvp.ref} … />; await qvp.search('الله')`
 * The page-bound API without passing a tag around.
 */
export function useQvp(): PageApi & { ref: React.RefObject<QvpPageViewHandle | null>; tag: () => number } {
  const ref = useRef<QvpPageViewHandle | null>(null);
  return useMemo(() => {
    const tag = () => ref.current?.tag() ?? -1;
    return { ref, tag, ...bindPage(tag) };
  }, []);
}

export default QvpPageView;
