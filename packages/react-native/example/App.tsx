/**
 * Mushaf Vector Reader — React Native example. Every visual state is engine state expressed as props
 * (`highlights`, `styles`, `theme`, `mask`, `reveal`); the native view only paints what the engine says.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { PanResponder, Pressable, ScrollView, StyleSheet, Text, TextInput, View, useWindowDimensions } from 'react-native';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import {
  QvpPageView, QvpAtlas, Qvp, useQvp, Sel, T, KIND, DECORATION, LAYER, rgba,
  type Word, type Deco, type Highlight, type StyleRule, type Theme, type Mask, type Reveal, type Match, type SelectionInfo, type PageInfo, type Stats, type HighlightMode, type Selector,
} from '@quran.ws/qvp-react-native';

const PAGES = [...Array.from({ length: 21 }, (_, i) => i + 1), 440, 441, 442, 443, 444, 445, 582, 604];
const pad3 = (n: number) => String(n).padStart(3, '0');
const THEMES = {
  light: { ink: '#231f20', paper: '#fffdf7', bg: '#f6f1e7', fg: '#1b1b1b', panel: '#fffdf7', line: '#d9d4c8' },
  sepia: { ink: '#3b2a14', paper: '#f3e7cf', bg: '#e9dcc3', fg: '#2b1d0c', panel: '#f3e7cf', line: '#cdbe9f' },
  dark: { ink: '#e8e4dc', paper: '#1e2126', bg: '#15171b', fg: '#e8e4dc', panel: '#1e2126', line: '#3a3f47' },
} as const;
type ThemeName = keyof typeof THEMES;

// ── tiny UI atoms ──
const FgCtx = React.createContext('#444');
function Btn({ label, on, onPress, small }: { label: string; on?: boolean; onPress: () => void; small?: boolean }) {
  const fg = React.useContext(FgCtx);
  return (
    <Pressable onPress={onPress} style={({ pressed }) => [st.btn, small && st.btnSmall, on && st.btnOn, pressed && { opacity: 0.6 }]}>
      <Text style={[st.btnText, { color: fg }, small && { fontSize: 11 }, on && { color: '#fff' }]}>{label}</Text>
    </Pressable>
  );
}
function Section({ title, fg }: { title: string; fg: string }) {
  return <Text style={[st.section, { color: fg }]}>{title.toUpperCase()}</Text>;
}
/** Pure-JS slider: no native module needed for the demo. */
function Slider({ label, min, max, value, step = 1, onChange, fg, fmt }: { label: string; min: number; max: number; value: number; step?: number; onChange: (v: number) => void; fg: string; fmt?: (v: number) => string }) {
  const width = useRef(1);
  const set = (x: number) => { const r = Math.max(0, Math.min(1, x / width.current)); const v = Math.round((min + r * (max - min)) / step) * step; onChange(+v.toFixed(3)); };
  const pan = useMemo(() => PanResponder.create({
    onStartShouldSetPanResponder: () => true, onMoveShouldSetPanResponder: () => true,
    onPanResponderGrant: e => set(e.nativeEvent.locationX), onPanResponderMove: e => set(e.nativeEvent.locationX),
  }), [min, max, step]);
  const r = (value - min) / (max - min);
  return (
    <View style={st.sliderRow}>
      <Text style={[st.small, { color: fg, width: 92 }]}>{label}</Text>
      <View style={st.sliderTrackBox} onLayout={e => (width.current = e.nativeEvent.layout.width)} {...pan.panHandlers}>
        <View style={st.sliderTrack} /><View style={[st.sliderFill, { width: `${r * 100}%` }]} /><View style={[st.sliderThumb, { left: `${r * 100}%` }]} />
      </View>
      <Text style={[st.small, { color: fg, width: 40, textAlign: 'right' }]}>{fmt ? fmt(value) : value}</Text>
    </View>
  );
}

export default function App() {
  return <SafeAreaProvider><Demo /></SafeAreaProvider>;
}

function Demo() {
  const qvp = useQvp();
  const { width: winW, height: winH } = useWindowDimensions();
  const [atlas, setAtlas] = useState<QvpAtlas | null>(null);
  const [pageNo, setPageNo] = useState(PAGES[0]);
  const [pageField, setPageField] = useState('1');
  const [info, setInfo] = useState<PageInfo | null>(null);
  const [theme, setTheme] = useState<ThemeName>('light');
  const [hlMode, setHlMode] = useState<HighlightMode>('both');
  const [hlMs, setHlMs] = useState(250);
  const [selWord, setSelWord] = useState<Word | null>(null);
  const [selAyah, setSelAyah] = useState<[number, number] | null>(null);
  const [selection, setSelection] = useState<SelectionInfo | null>(null);
  const [ayahInfo, setAyahInfo] = useState<{ text: string; count: number; complete: boolean } | null>(null);
  const [pathOn, setPathOn] = useState<Map<number, Selector>>(new Map());
  const [query, setQuery] = useState('');
  const [matches, setMatches] = useState<Match[]>([]);
  const [tajwid, setTajwid] = useState(false);
  const [hideMarks, setHideMarks] = useState(false);
  const [gold, setGold] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [playIdx, setPlayIdx] = useState(0);
  const [maskMode, setMaskMode] = useState<'hide' | 'block'>('hide');
  const [mask, setMask] = useState<Mask | null>(null);
  const [revealOn, setRevealOn] = useState(false);
  const [revealPosition, setRevealAt] = useState(-1);
  const [revealStepCount, setRevealSteps] = useState(0);
  const [lineSpacing, setLineSpacing] = useState(1);
  const [fillHeight, setFillHeight] = useState(false);
  const [padTop, setPadTop] = useState(12);
  const [padBottom, setPadBottom] = useState(12);
  const [meta, setMeta] = useState('');
  const [stats, setStats] = useState<Stats | null>(null);
  const [note, setNote] = useState('');
  const pendingAyah = useRef<[number, number] | null>(null);
  const pageViewH = useRef(0);
  const th = THEMES[theme];

  useEffect(() => { QvpAtlas.load('asset://pages/atlas.qva').then(setAtlas).catch(e => setNote('no atlas: ' + e.message)); }, []);
  useEffect(() => { const id = setInterval(() => qvp.stats().then(s => s && setStats(s)).catch(() => {}), 1000); return () => clearInterval(id); }, [qvp]);
  const flash = (s: string) => { setNote(s); setTimeout(() => setNote(''), 3000); };

  // ── page navigation (atlas) ──
  const loadPage = (n: number) => {
    const target = PAGES.includes(n) ? n : PAGES.reduce((a, b) => (Math.abs(b - n) < Math.abs(a - n) ? b : a));
    setPageNo(target); setPageField(String(target));
  };
  const stepPage = (d: number) => { const i = PAGES.indexOf(pageNo); if (PAGES[i + d] != null) loadPage(PAGES[i + d]); };
  const goto = async (s: string) => {
    if (!atlas) return;
    let m: RegExpExecArray | null;
    if ((m = /^(\d+):(\d+)/.exec(s))) { const pg = await atlas.pageOf(+m[1], +m[2]); if (pg) { pendingAyah.current = [+m[1], +m[2]]; loadPage(pg); } return; }
    if ((m = /^juz\s*(\d+)/i.exec(s))) { const j = await atlas.juz(+m[1]); if (j) loadPage(j.page); return; }
    const su = await atlas.searchSurahs(s); if (su.length) loadPage(su[0].page);
  };

  // ── selection ──
  const selectWord = useCallback((w: Word | null) => {
    setSelAyah(null); setAyahInfo(null); setPathOn(new Map()); qvp.clearSelection().catch(() => {});
    setSelWord(prev => (w == null || prev?.index === w.index ? null : w));
  }, [qvp]);
  const selectAyah = useCallback(async (s: number, a: number) => {
    setSelWord(null); setPathOn(new Map()); qvp.clearSelection().catch(() => {});
    setSelAyah([s, a]);
    const [text, wc] = await Promise.all([qvp.text(T.ayah(s, a)), qvp.ayahWordCount(s, a)]);
    setAyahInfo({ text, ...wc });
  }, [qvp]);

  const onPageLoad = useCallback(async (e: PageInfo) => {
    setInfo(e); setSelWord(null); setSelAyah(null); setAyahInfo(null); setSelection(null); setPathOn(new Map()); setRevealOn(false); setRevealAt(-1); setMask(null);
    const [surahs, divisions, keys] = await Promise.all([qvp.surahs(), qvp.divisions(), qvp.ayahKeys()]);
    let t = 'surahs: ' + surahs.map(s => `${s.number}${s.latin ? ' ' + s.latin : ''}${s.hasBanner ? ' (banner)' : ''}`).join(', ');
    if (divisions.length) t += '\nstarts here: ' + divisions.map(d => `${d.division} ${d.number} at ${d.surah}:${d.ayah}`).join(', ');
    if (atlas && keys.length) { const j = await atlas.juzOf(keys[0].surah, keys[0].ayah); if (j) { const pr = await atlas.pagesOfJuz(j); t += `\njuz ${j} · pages ${pr ? pr.join('–') : ''}`; } }
    t += '\nayahs: ' + keys.map(k => k.ayahKey).join(' ');
    setMeta(t);
    if (query.trim()) setMatches(await qvp.search(query.trim()));
    if (pendingAyah.current) { const [s, a] = pendingAyah.current; pendingAyah.current = null; selectAyah(s, a); }
    if (playing) setPlayIdx(0);
  }, [qvp, atlas, query, playing, selectAyah]);

  // ── search ──
  useEffect(() => {
    let live = true;
    const q = query.trim();
    if (!q) { setMatches([]); return; }
    qvp.search(q).then(m => live && setMatches(m)).catch(() => {});
    return () => { live = false; };
  }, [query, qvp]);

  // ── follow words: one handle, moveHighlight via the prop ──
  useEffect(() => {
    if (!playing) return;
    const id = setInterval(() => {
      setPlayIdx(i => {
        if (info && i + 1 >= info.nWords) { const k = PAGES.indexOf(pageNo); if (PAGES[k + 1] != null) { loadPage(PAGES[k + 1]); return 0; } setPlaying(false); return i; }
        return i + 1;
      });
    }, 320);
    return () => clearInterval(id);
  }, [playing, info, pageNo]);

  // ── declarative engine state ──
  const highlights = useMemo<Highlight[]>(() => {
    const out: Highlight[] = [];
    if (selWord) out.push({ id: 'sel', target: T.word(selWord.index), style: { mode: hlMode, ink: '#1a73e8', band: rgba('#1a73e8', 0.18), radius: 1.5, ms: hlMs, layer: LAYER.SELECTION } });
    if (selAyah) out.push({ id: 'ayah', target: T.ayah(selAyah[0], selAyah[1]), style: { mode: hlMode, ink: '#0a7d32', band: rgba('#0a7d32', 0.14), radius: 1.5, ms: hlMs, layer: LAYER.SELECTION } });
    if (matches.length) out.push({ id: 'search', target: T.words(matches.map(m => m.word)), style: { mode: 'both', ink: '#c62828', band: rgba('#c62828', 0.12), height: 'ink', padY: 1, radius: 1, ms: hlMs } });
    if (playing) out.push({ id: 'play', target: T.word(playIdx), style: { mode: hlMode, ink: '#d81b60', band: rgba('#d81b60', 0.14), radius: 1.5, ms: hlMs } });
    return out;
  }, [selWord, selAyah, matches, playing, playIdx, hlMode, hlMs]);
  const styles = useMemo<StyleRule[]>(() => {
    const out: StyleRule[] = [];
    if (hideMarks) out.push({ id: 'hide-marks', selector: Sel.kind(KIND.MARK), hide: true });
    if (gold) out.push({ id: 'ayahMarks', selector: Sel.decoration(DECORATION.AYAH_MARK), color: '#b8860b', ms: 300, layer: LAYER.THEME + 1 });
    for (const [i, selector] of pathOn) out.push({ id: `path:${i}`, selector, color: '#ef6c00', ms: 200, layer: LAYER.TOP });
    return out;
  }, [hideMarks, gold, pathOn]);
  const markTheme = useMemo<Theme | null>(() => (tajwid ? { diacritics: '#1a73e8', dots: '#c62828', waqf: '#0a7d32', sifr: '#ef6c00', ms: 200 } : null), [tajwid]);
  const reveal = useMemo<Reveal | null>(() => (revealOn ? { lit: 2, grey: theme === 'dark' ? '#4a4f57' : '#c9c4b8', ink: th.ink, ms: 150, at: revealPosition } : null), [revealOn, revealPosition, theme, th.ink]);

  const maskAyah = () => {
    const target = selAyah ? T.ayah(selAyah[0], selAyah[1]) : selWord ? T.ayah(selWord.surah, selWord.ayah) : 'page';
    setMask({ target, mode: maskMode, blockColor: th.line });
  };
  const copySelection = async () => {
    let text: string;
    if (selection && selection.words.length) text = selection.textWithCitation;
    else if (selAyah && ayahInfo) text = `${ayahInfo.text} (${selAyah[0]}:${selAyah[1]})`;
    else if (selWord) text = `${selWord.text} (${await qvp.citation([selWord.index])})`;
    else return;
    flash('copied: ' + text);
  };
  const cropSelection = async () => {
    const target = selection && selection.words.length ? T.words(selection.words) : selAyah ? T.ayah(selAyah[0], selAyah[1]) : selWord ? T.word(selWord.index) : null;
    if (!target) return;
    const [svg, box] = await Promise.all([qvp.cropSvg(target, { pad: 3, keepAyahMarks: true, background: th.paper }), qvp.cropBounds(target, { pad: 3, keepAyahMarks: true })]);
    if (svg && box) flash(`SVG ${(svg.length / 1024).toFixed(0)} KB · box ${(box.x1 - box.x0).toFixed(0)}×${(box.y1 - box.y0).toFixed(0)} units · ayahMark ${box.ayahMarkDecoration >= 0 ? 'kept' : 'no'}`);
  };
  const clearAll = () => {
    setTajwid(false); setHideMarks(false); setGold(false); setPlaying(false); setMask(null); setRevealOn(false); setRevealAt(-1);
    setQuery(''); setMatches([]); selectWord(null);
  };
  const leadingToFill = async () => {
    if (!info) return;
    setFillHeight(false); setLineSpacing(1);
    setLineSpacing(await qvp.layoutLineSpacingToFill());
  };

  // ── selection panel ──
  const selMain = selection && selection.words.length > 1 ? selection.text : selWord ? selWord.text : ayahInfo ? ayahInfo.text : '—';
  const selInfo = selection && selection.words.length > 1 ? `selection · ${selection.words.length} words · ${selection.citation}`
    : selWord ? `wordKey ${selWord.wordKey} · line ${selWord.line} · ${selWord.nPaths} paths\n${selWord.forms.rasmImlai ? `rasmImlai ${selWord.forms.rasmImlai} · search ${selWord.forms.search}\n` : ''}${selWord.label}`
    : selAyah && ayahInfo ? `ayah ${selAyah[0]}:${selAyah[1]} · ${ayahInfo.count} words${ayahInfo.isComplete ? '' : ' (continues on another page)'}` : '';

  const hud = stats ? [
    `engine v${stats.engineVersion} · JNI over qvp.h${atlas ? ' · atlas' : ''}`,
    `page ${pad3(pageNo)}      ${(stats.bytes / 1024).toFixed(0)} KB, load ${stats.loadMs.toFixed(2)} ms`,
    info ? `content       ${info.nWords} words · ${info.nPaths} paths · ${info.nLines} lines` : '',
    `base layer    ${stats.basePaths} paths in ${stats.baseMs.toFixed(2)} ms (cached)`,
    `overlay       ${stats.overlayPaths} styled + ${stats.bands} bands in ${stats.overlayMs.toFixed(2)} ms`,
    `hit-test      ${stats.hitUs.toFixed(1)} µs · ${stats.styleHandles} handles · ${stats.highlightHandles} highlights`,
    `layout        ${fillHeight ? 'fill height' : `spacing ×${lineSpacing.toFixed(2)}`} · lineSpacing ${(stats.layout?.lineSpacing ?? 0).toFixed(1)} u`,
  ].filter(Boolean).join('\n') : '';

  return (
    <FgCtx.Provider value={th.fg}>
    <SafeAreaView style={[st.root, { backgroundColor: th.bg }]} edges={['top', 'bottom']}>
      <View style={st.bar}>
        <TextInput style={[st.input, { flex: 1, color: th.fg, borderColor: th.line }]} placeholder="2:255 · Yasin · juz 30" placeholderTextColor="#999" onSubmitEditing={e => goto(e.nativeEvent.text.trim())} returnKeyType="go" />
        <Btn label="◀" onPress={() => stepPage(-1)} />
        <TextInput style={[st.input, { width: 56, textAlign: 'center', color: th.fg, borderColor: th.line }]} value={pageField} onChangeText={setPageField} keyboardType="number-pad" onSubmitEditing={() => { const n = parseInt(pageField, 10); if (n) loadPage(n); }} />
        <Btn label="▶" onPress={() => stepPage(+1)} />
      </View>

      <QvpPageView
        ref={qvp.ref}
        style={st.page}
        onLayout={e => (pageViewH.current = e.nativeEvent.layout.height)}
        pageUri={`asset://pages/${pad3(pageNo)}.qvp`}
        wordsUri={`asset://pages/${pad3(pageNo)}.words.json`}
        padTop={padTop} padBottom={padBottom} padSide={8}
        lineSpacing={lineSpacing} fillHeight={fillHeight}
        paperColor={th.paper} defaultInk={th.ink}
        theme={markTheme} styles={styles} highlights={highlights} mask={mask} reveal={reveal}
        onPageLoad={onPageLoad}
        onWordTap={e => selectWord(e.word)}
        onDecorationTap={(e: { decoration: Deco }) => { if (e.decoration.ayah) selectAyah(e.decoration.surah, e.decoration.ayah); }}
        onEmptyTap={() => selectWord(null)}
        onSelectionChanged={e => { setSelection(e); if (e.words.length > 1) { setSelWord(null); setSelAyah(null); setAyahInfo(null); } }}
        onRevealChanged={e => setRevealSteps(e.steps)}
        onError={e => flash('error: ' + e.message)}
      />

      <ScrollView style={[st.panel, { backgroundColor: th.panel, height: Math.min(330, winH * 0.38) }]} contentContainerStyle={{ padding: 12, paddingBottom: 24 }} keyboardShouldPersistTaps="handled">
        {!!note && <Text style={[st.small, { color: '#c62828' }]}>{note}</Text>}
        <Section title="Search this page" fg={th.fg} />
        <TextInput style={[st.input, { color: th.fg, borderColor: th.line, textAlign: 'right' }]} placeholder="الله · الرحمان" placeholderTextColor="#999" value={query} onChangeText={setQuery} />
        {query.trim() !== '' && matches.length === 0 && <Text style={[st.small, { color: th.fg, opacity: 0.6 }]}>no match on this page</Text>}
        {matches.slice(0, 8).map(m => (
          <Pressable key={m.word} onPress={async () => selectWord((await qvp.word(m.word))!)}><Text style={[st.result, { color: th.fg }]}>{m.text}  <Text style={st.small}>{m.wordKey}{m.isLooseMatch ? ' ~' : ''}</Text></Text></Pressable>
        ))}

        <Section title="Selection" fg={th.fg} />
        <Text style={[st.selWord, { color: th.fg }]}>{selMain}</Text>
        <Text style={[st.small, { color: th.fg }]}>{selInfo}</Text>
        {selWord && (
          <ScrollView horizontal showsHorizontalScrollIndicator={false} style={{ marginVertical: 4 }}>
            {selWord.paths.map(p => {
              const label = p.kind === KIND.MARK ? `${p.markName} #${p.nthMark}` : p.kindName;
              const sel = p.kind === KIND.MARK && p.nthMark >= 0 ? Sel.wordMark(selWord.index, p.nthMark) : Sel.path(p.index);
              return <Btn key={p.index} small label={label} on={pathOn.has(p.index)} onPress={() => setPathOn(prev => { const m = new Map(prev); if (m.has(p.index)) m.delete(p.index); else m.set(p.index, sel); return m; })} />;
            })}
          </ScrollView>
        )}
        <View style={st.row}><Btn label="Copy + citation" onPress={copySelection} /><Btn label="Crop → SVG" onPress={cropSelection} /></View>
        <Text style={[st.small, { color: th.fg, opacity: 0.7 }]}>Tap a word · long-press and drag to select · tap an ayah mark · chips recolour one path (e.g. 2nd diacritic)</Text>

        <Section title="Highlights (engine-animated)" fg={th.fg} />
        <View style={st.row}>
          {(['both', 'band', 'ink'] as HighlightMode[]).map(m => <Btn key={m} small label={m === 'both' ? 'band + ink' : m} on={hlMode === m} onPress={() => setHlMode(m)} />)}
          <Btn label={playing ? '■ Stop' : '▶ Follow words'} on={playing} onPress={() => { if (!playing) setPlayIdx(selWord ? selWord.index : 0); setPlaying(!playing); }} />
        </View>
        <Slider label="fade ms" min={0} max={800} value={hlMs} step={10} onChange={setHlMs} fg={th.fg} />

        <Section title="Styling (each toggle is one engine handle)" fg={th.fg} />
        <View style={st.row}>
          <Btn label="Mark colours" on={tajwid} onPress={() => setTajwid(!tajwid)} />
          <Btn label="Hide marks" on={hideMarks} onPress={() => setHideMarks(!hideMarks)} />
          <Btn label="Gold ayah marks" on={gold} onPress={() => setGold(!gold)} />
        </View>
        <View style={st.row}>
          <Text style={[st.small, { color: th.fg }]}>Theme </Text>
          {(Object.keys(THEMES) as ThemeName[]).map(t => <Btn key={t} small label={t} on={theme === t} onPress={() => setTheme(t)} />)}
          <Btn label="Clear all" onPress={clearAll} />
        </View>

        <Section title="Memorisation" fg={th.fg} />
        <View style={st.row}>
          <Btn label="Mask ayah" onPress={maskAyah} />
          <Btn small label="hide" on={maskMode === 'hide'} onPress={() => setMaskMode('hide')} />
          <Btn small label="block" on={maskMode === 'block'} onPress={() => setMaskMode('block')} />
          <Btn label="Reveal" onPress={() => qvp.unmaskNext(1)} />
          <Btn label="Hide back" onPress={() => qvp.maskBack(1)} />
          <Btn label="Unmask" onPress={() => setMask(null)} />
        </View>
        <View style={st.row}>
          <Btn label="Greyed page" on={revealOn} onPress={() => { setRevealAt(-1); setRevealOn(!revealOn); }} />
          {revealOn && <Slider label="position" min={-1} max={Math.max(0, revealStepCount - 1)} value={revealPosition} onChange={setRevealAt} fg={th.fg} fmt={v => `${v + 1}/${revealStepCount}`} />}
        </View>

        <Section title="Layout (engine)" fg={th.fg} />
        <Slider label="line spacing" min={1} max={2.2} step={0.01} value={lineSpacing} onChange={v => { setLineSpacing(v); setLineGap(0); setFillHeight(false); }} fg={th.fg} fmt={v => '×' + v.toFixed(2)} />
        <Slider label="pad top" min={0} max={120} value={padTop} onChange={setPadTop} fg={th.fg} />
        <Slider label="pad bottom" min={0} max={120} value={padBottom} onChange={setPadBottom} fg={th.fg} />
        <View style={st.row}>
          <Btn label="Fill screen height" on={fillHeight} onPress={() => setFillHeight(!fillHeight)} />
          <Btn label="Leading to fill" onPress={leadingToFill} />
        </View>

        <Section title="Page" fg={th.fg} />
        <Text style={[st.small, { color: th.fg }]}>{meta}</Text>
        <Section title="Engine" fg={th.fg} />
        <Text style={[st.mono, { color: th.fg }]}>{hud}</Text>
      </ScrollView>
    </SafeAreaView>
    </FgCtx.Provider>
  );
}

const st = StyleSheet.create({
  root: { flex: 1 },
  bar: { flexDirection: 'row', alignItems: 'center', paddingHorizontal: 8, paddingVertical: 4, gap: 4 },
  page: { flex: 1, alignSelf: 'stretch' },
  panel: { flexGrow: 0, flexShrink: 0, borderTopWidth: StyleSheet.hairlineWidth, borderTopColor: '#8884' },
  input: { borderWidth: 1, borderRadius: 6, paddingHorizontal: 8, paddingVertical: 6, fontSize: 14 },
  row: { flexDirection: 'row', flexWrap: 'wrap', alignItems: 'center', gap: 4, marginVertical: 2 },
  btn: { paddingHorizontal: 10, paddingVertical: 6, borderRadius: 6, backgroundColor: '#8883', margin: 2 },
  btnSmall: { paddingHorizontal: 8, paddingVertical: 4 },
  btnOn: { backgroundColor: '#1a73e8' },
  btnText: { fontSize: 12, color: '#444' },
  section: { fontSize: 10.5, fontWeight: 'bold', opacity: 0.7, marginTop: 12, marginBottom: 2 },
  small: { fontSize: 11 },
  result: { fontSize: 16, textAlign: 'right', paddingVertical: 2 },
  selWord: { fontSize: 26, textAlign: 'right' },
  mono: { fontFamily: 'monospace', fontSize: 10.5 },
  sliderRow: { flexDirection: 'row', alignItems: 'center', marginVertical: 4, flex: 1, minWidth: 200 },
  sliderTrackBox: { flex: 1, height: 28, justifyContent: 'center' },
  sliderTrack: { height: 4, borderRadius: 2, backgroundColor: '#8886' },
  sliderFill: { position: 'absolute', left: 0, height: 4, borderRadius: 2, backgroundColor: '#1a73e8' },
  sliderThumb: { position: 'absolute', width: 16, height: 16, borderRadius: 8, backgroundColor: '#1a73e8', marginLeft: -8 },
});
