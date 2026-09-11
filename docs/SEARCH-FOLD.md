# The Quran.ws search fold

**Status:** proposed, one PR per repository, none merged.
**Scope:** every block that answers a Qurʾānic text query — Quran Text, Quran SVG
Elements, Quran Engine. Quran Tajweed is explicitly *out* of scope; see §7.

A developer who uses two of our blocks must get the same answer to the same
query. Today they do not.

---

## 1. The bug that started this

`quran-text`'s `fold()` expands the dagger alif (U+0670) into a full alif, so
the query a user actually types returns nothing:

```js
m.search("الرحمن")   // → 0     ← what a phone keyboard produces
m.search("الرحمان")  // → 45    ← what fold() requires
```

Verified against `lib/js/data/hafs.json` at `646f528`. Quran SVG Elements and
Quran Engine do the opposite: both key `ٱلرَّحۡمَٰنِ` as `الرحمن`, and Engine's own
test asserts it (`crates/qvp-core/src/text.rs:184`).

## 2. What each block does today

Measured by running each implementation, not read from a README.

| | dagger alif U+0670 | hamza forms | ة → ه | ى → ي | tatweel, harakāt, waqf signs | key built from |
|---|---|---|---|---|---|---|
| **Quran Text** `fold()` | **→ full alif** | ٱأإآ→ا only; ء ئ ؤ kept | no | yes | stripped by codepoint ranges | the **ʿUthmānī** text |
| **Quran SVG Elements** `search` | **stripped** | none — kept as written | no | no | stripped by Unicode **category** | the **imlāʾī** spelling |
| **Quran Engine** `normalize_query` | **stripped** | أإآٱ→ا, ؤ→و, ئ→ي | **yes** | yes | stripped by codepoint ranges | Elements' `search` field |
| **Quran Tajweed** `normalize()` | → full alif | recovers borne hamza | no | no | dropped as decoration | not a search path |

Three blocks, three answers. Engine additionally ships a `loose_key` fallback
that drops bare alif and hamza — added, its comment says, so that "a typed
الرحمان finds the printed الرحمن". The problem was already known in one repo.

## 3. Why neither mechanical rule is correct

The obvious fix is to pick one rule for U+0670. Both are wrong, and the corpus
says so. Against all 77,356 Ḥafṣ words that carry an imlāʾī spelling:

| mechanical rule | words whose key ≠ what a user types |
|---|---|
| U+0670 → full alif (Quran Text today) | **5,866** |
| U+0670 stripped (Engine/Elements today) | **7,807** |

```
fold-to-alif is wrong on:  ٱلرَّحۡمَٰنِ → الرحمان   (user types الرحمن)
                            ذَٰلِكَ    → ذالك      (user types ذلك)
stripping is wrong on:     ٱلۡعَٰلَمِينَ → العلمين   (user types العالمين)
                            مَٰلِكِ     → ملك       (user types مالك)
```

The dagger alif is written as a full alif in modern spelling in most words and
omitted in a small high-frequency set — and that set is not closed: 1,101
distinct wrong-key pairs, the top 25 covering under half the occurrences. No
codepoint rule reproduces it, because the choice is **orthographic, not
mechanical**.

The imlāʾī spelling already records that choice, per word. This is the same
conclusion Abdullah reached in Elements on 2026-08-29
(`tools/assign_words.py:3472`) and it is the org's answer; `quran-text` never
got the memo, though it already ships the data as `rasm_imlai`.

A second, independent proof. The ʿUthmānī sometimes carries a hamza on a
tatweel, and stripping marks deletes the consonant outright:

```
uthmani أَنۢبِـُٔونِي  → strip → أنبوني     ← no hamza left; nobody types this
imlai   أنبئوني              → أنبئوني     ← what a user types
```

## 4. The specification

Three functions. Order of operations is load-bearing and stated.

### 4.1 `search_key(text)` — index time, stored

1. Normalise to **NFC**.
2. Remove every character whose Unicode **general category** is one of
   `Mn`, `Me`, `Lm`, `Sk`, `So`, `Cf`.
3. Collapse runs of whitespace to a single space; trim.

Nothing is folded. Category `Mn` covers all harakāt, tanwīn, the dagger alif
U+0670, the Qurʾānic annotation signs U+06D6–U+06ED and U+0610–U+061A; `Lm`
covers tatweel U+0640; `So` covers rub el hizb U+06DE and sajdah U+06E9; `Cf`
covers ZWJ/ZWNJ/RLM/BOM. A category test rather than a hand-written range list —
there is nothing to keep in sync as Unicode adds Qurʾānic marks, and the two
groups are disjoint by category.

**The input is the imlāʾī spelling wherever the muṣḥaf has one.** Never the
ʿUthmānī. §3 is why.

### 4.2 `match_fold(text)` — applied to BOTH sides at match time

1. Normalise to **NFKC** (folds Arabic presentation forms; verified to change
   no word in the Ḥafṣ corpus).
2. Remove the same six categories as §4.1.
3. Map letters, single pass, no rule feeding another:

   | from | to |
   |---|---|
   | ٱ U+0671, أ U+0623, إ U+0625, آ U+0622 | ا U+0627 |
   | ى U+0649, ئ U+0626 | ي U+064A |
   | ؤ U+0624 | و U+0648 |
   | ة U+0629 | ه U+0647 |
   | Arabic-Indic digits U+0660–0669, U+06F0–06F9 | ASCII 0–9 |

   ء U+0621 is **kept** — it is a letter a user types (شيء, علماء).
4. Collapse whitespace.

Step 2 must precede step 3: a mark sitting between a bearer and its hamza
would otherwise block the letter mapping.

Both the stored key and the query go through this. Folding only the query
leaves `أنعمت` (key) unreachable from `انعمت` (query) — 4,385 of 14,897
distinct keys, 29%, contain a letter users type variably.

### 4.3 `loose_key(text)` — fallback only, results flagged

`match_fold`, then drop ا U+0627 and ء U+0621.

Run **only when the strict pass returns nothing**, and mark every result as
loose so a UI can say so. It is deliberately lossy: it merges 1,217 key groups
covering 2,644 distinct words — `كاتب`/`كتاب`/`كتب` all collapse to `كتب`. That
is an acceptable price for a second chance and an unacceptable one for a first.
This is Engine's existing design, and it is right.

## 5. What this fixes

```
query        strict   loose
الرحمن           45      45     ← was 0 in quran-text
الرحمان           0      45     ← reaches the same 45 via the flagged fallback
العالمين         61      61     ← the case that defeats mechanical stripping
مالك              2      45
انعمت             7       7     ← hamza-folded to أنعمت
أنعمت             7       7
```

Both spellings of the motivating query reach the same 45 āyāt.

## 6. Sources with no imlāʾī — `search_variants()`

Only Ḥafṣ has an imlāʾī column, and it is not complete even there (76 words of
77,432 have none). The cross-riwāyah word index has none at all, and
`quran-text` rightly refuses to derive one for the other six riwāyāt: that
would be the project asserting a spelling no source states
(`pipeline/qurantext/rasm_imlai.py`).

For those sources, index **every spelling the word might reasonably be typed
as** rather than guessing one:

```
search_variants(text) = { match_fold(v) , without ء , ء→ي }
                        for v in { text , text with U+0670 → ا }
```

Six candidates at most, usually two or three. Measured against Ḥafṣ, where the
true imlāʾī spelling is known, this set contains it for **97.83%** of the
77,356 words (the single dagger-alif rule alone reaches 96.46%; the hamza
variants carry the rest). It costs 17,625 distinct keys against imlāʾī's
14,678 — a 20% larger index, no runtime cost, and no over-collision of the kind
`loose_key` has.

The residual 2.2% differ orthographically in ways no codepoint rule reaches:

```
ٱلصَّلَوٰةَ  → الصلاة    (variants give الصلوه / الصلواه)
ٱلسَّمَٰوَٰتِ → السموات   (variants give السماوات / السموت)
```

**What still needs Abdullah.** Nothing here is blocking, but two calls are his:
whether to source an imlāʾī text for the other six riwāyāt (the only thing that
closes the last 2.2%), and whether 97.83% with a 20% larger index is the right
trade for the word index, or whether those riwāyāt should simply document
loose-fallback recall instead. The PRs ship `search_variants` as the default and
say so in the open.

## 7. Quran Tajweed is out of scope

Its `normalize()` also maps U+0670 to a full alif, and that is **correct there
and must not be "fixed"**: a dagger alif is phonetically a long ā, and tajweed
rules match on how a word is *read*. It exposes no search API. The one thing it
owes this spec is a note saying its normaliser is phonetic and is not a search
fold — otherwise someone will reuse it as one.

## 9. What the open-source implementations do

Checked against source, not blog posts, and the live quran.com API was probed
directly. This section exists because the decisions that recur across
independently built systems are the ones that have survived real users.

### The dagger alif, project by project

| | U+0670 | evidence |
|---|---|---|
| **quran.com / Quran Foundation** | **dropped** | `icu_folding` in the analyzer chain; `DiacriticFolding.txt` carries `0670>` (delete). Confirmed live: `لكن` returns 6 hits all containing U+0670, `لكان` returns 6 hits all containing a real alif — **disjoint sets**, so it cannot be folding to alif. |
| **QUL / Tarteel** | **dropped** in `app/services/search/arabic_normalizer.rb` — but **folded to alif** in `lib/export_quran_fts.rb`, which feeds the SQLite FTS tables. One organisation, two policies. |
| **Tanzil** | **kept** in `simple`/`simple-plain`/`simple-min`; **dropped** in `simple-clean` |
| **alfanous** | **folded to a full alif**, deliberately — its docstring says "so that words like سَمَّٰكُمُ normalize to سماكم matching standard user input" |
| **Quranic Arabic Corpus** | **preserved** as distinct — its own Buckwalter symbol `` ` ``, never `A` |
| **Lucene `ArabicNormalizer`** | **untouched** — the whole filter is five maps (آأإ→ا, ى→ي, ة→ه) and two deletions (tatweel, U+064B–U+0652). It cannot match a ʿUthmānī word against a typed one at all. |

So alfanous is the one project that did what `quran-text` did, and it is the
outlier. The largest live system drops it.

### The premise needed correcting, and the correction strengthens §3

The obvious framing — "ʿUthmānī `الرحمن` versus imlāʾī `الرحمان`" — is wrong, and
worth stating because it is the intuitive way to get this backwards. **Tanzil's
Imlaei script does not write `الرحمان`.** It writes `الرَّحْمَـٰنِ`, keeping U+0670 on
a tatweel carrier. `الرحمان` is a machine artifact of folding, not a spelling
anyone publishes.

What Tanzil actually does is the whole point: converting ʿUthmānī to Simple
turns roughly 6,500 of 9,838 dagger alifs into a real U+0627 and **leaves 3,330
alone** — and the ~385 word-forms that keep it are precisely the ones modern
orthography also writes without an alif (`علىٰ` 428, `ذٰلك` 280, `إلىٰ` 265,
`هٰذا` 190, `الرحمٰن` 157).

That is **word-class-selective, not mechanical** — independently, in the
canonical Imlaei source, the same conclusion §3 reaches from our own corpus.
Our `rasm_imlai` and Tanzil's Simple are doing the same job. It is the single
strongest confirmation in this document, and it was not designed for.

### What recurs, and so is load-bearing

Present in three or more independently built systems:

1. **Strip tatweel U+0640** — Lucene, ICU, QUL, alfanous, Tanzil. No dissent.
2. **Strip U+064B–U+0652.** Universal.
3. **ة→ه and ى→ي** — Lucene, ICU, QUL, alfanous, Larkey 2002, Kadri & Nie 2006,
   and Tanzil's documented behaviour (`نعمت` matches `نعمة`). Six confirmations,
   the most corroborated rule in the set.
4. **Alif-seated hamzas أ إ آ → ا.** Universal.
5. **Marks stripped before letters mapped** — §4.2 step 2 before step 3.
6. **The same function at index and query time.** QUL calls one normaliser on
   both sides; quran.com has a single `analyzer:` with no `search_analyzer`
   override, confirmed by three equivalent queries returning identical results.
   **No project normalises only one side.** §4.2 does this; it is why Elements'
   published recipe needed fixing.
7. **Keep the original beside the folded form.** alfanous ships an unfolded
   `standard_full` field; QUL keeps four scripts plus an offset map for
   highlighting; quran.com keeps un-normalised `lemma`/`stem`/`root` beside
   normalised ones. **Nobody destroys the original.** §4.1 stores unfolded and
   folds at match time for exactly this reason.

### Where they genuinely disagree, and what we picked

- **ؤ and ئ.** QUL → و / ي. alfanous → ء. Lucene, ICU, Larkey → unchanged.
  No consensus. **We follow QUL** (و / ي), which is also what Engine already
  did. A deliberate pick, not a default.
- **ة→ه and ى→ي word-final only?** Larkey 2002 and Kadri & Nie 2006 both
  restrict them to word-final position; every software implementation applies
  them unconditionally. **We follow the software consensus.** The literature's
  corpora were unvocalised newswire, where the trade-off is different.
- **U+0671 alef wasla.** QUL and alfanous both fold it; Lucene and ICU both
  leave it (it has no canonical decomposition). Both hand-rolled *Qurʾān*
  normalisers independently added the rule the generic Arabic tooling lacks,
  which is itself the signal. **We fold it.**

### The experiment that forces `search_variants`

Folded under each verified scheme, no single policy matches both the ʿUthmānī
word and both spellings a user might type:

| scheme | ʿUthmānī `ٱلرَّحۡمَٰنِ` | typed `الرحمن` | typed `الرحمان` |
|---|---|---|---|
| Lucene ArabicNormalizer | `الرحمٰن` | ✗ | ✗ |
| ICU folding / QUL | `الرحمن` | **✓** | ✗ |
| alfanous | `الرحمان` | ✗ | **✓** |

Drop wins the typed query and loses `الرحمان`; fold does the reverse. **The only
policy that serves both is to index more than one form** — §6's
`search_variants`, or §4.3's flagged fallback. That conclusion is forced by the
data rather than chosen, and it is reassuring that two of our repos had already
arrived at the fallback half of it independently.

### One premise, now with a citation

A standard Arabic **phone** keyboard cannot produce U+0670: it appears nowhere
in the AOSP/LineageOS `rowkeys_arabic*.xml` layouts, long-press alternates
included. On desktop it is third-level (AltGr) in xkeyboard-config's
`symbols/ara`. So the spelling users can actually type is the one without it —
which is what §4 keys on.

### Retrieval evidence

Larkey, Ballesteros & Connell (SIGIR 2002) measured normalisation — diacritic
removal plus أإآ→ا, final ى→ي, final ة→ه — at **+23.1%** average precision
monolingual (.194 → .238) and **+133%** cross-language. Their corpus was
unvocalised newswire, so "remove diacritics" cost them nothing that it costs us;
treat the number as directional, not transferable. No paper was found that
ablates individual normalisation rules, so per-rule deltas would be invention.

## 8. Conformance vector

Every input below is copied from a real data file; none is typed by hand. The
machine-readable copy is `conformance.json` in each PR.

| rule | input | source | `search_key` | `match_fold` | `loose_key` |
|---|---|---|---|---|---|
| dagger alif U+0670 is a MARK: stripped, never expanded | `ٱلرَّحۡمَٰنِ` | quran-text hafs rasm_uthmani 1:1:3 | `ٱلرحمن` | `الرحمن` | `لرحمن` |
|   ...the same word's imlai, the search-key source | `الرحمن` | quran-text hafs rasm_imlai 1:1:3 | `الرحمن` | `الرحمن` | `لرحمن` |
| dagger alif where imlai writes a FULL alif | `ٱلۡعَٰلَمِينَ` | quran-text hafs rasm_uthmani 1:2:4 | `ٱلعلمين` | `العلمين` | `لعلمين` |
|   ...its imlai | `العالمين` | quran-text hafs rasm_imlai 1:2:4 | `العالمين` | `العالمين` | `لعلمين` |
| alef wasla U+0671 | `ٱللَّهِ` | quran-text hafs rasm_uthmani | `ٱلله` | `الله` | `لله` |
| hamza above U+0623 | `أَنۡعَمۡتَ` | quran-text hafs rasm_uthmani | `أنعمت` | `انعمت` | `نعمت` |
| madda U+0622 | `ٱلضَّآلِّينَ` | quran-text hafs rasm_uthmani | `ٱلضآلين` | `الضالين` | `لضلين` |
| ta marbuta U+0629 | `ٱلصَّلَوٰةَ` | quran-text hafs rasm_uthmani | `ٱلصلوة` | `الصلوه` | `لصلوه` |
| waw hamza U+0624 | `يُؤۡمِنُونَ` | quran-text hafs rasm_uthmani | `يؤمنون` | `يومنون` | `يومنون` |
| yeh hamza U+0626 | `أُوْلَٰٓئِكَ` | quran-text hafs rasm_uthmani | `أولئك` | `اوليك` | `وليك` |
| bare hamza U+0621 | `سَوَآءٌ` | quran-text hafs rasm_uthmani | `سوآء` | `سواء` | `سو` |
| shadda + tanwin | `رَغَدًا` | quran-text hafs rasm_uthmani | `رغدا` | `رغدا` | `رغد` |
| tatweel-borne hamza: stripping the uthmani LOSES the consonant | `أَنۢبُِٔونِي` | quran-text hafs rasm_uthmani 2:31 | `أنبوني` | `انبوني` | `نبوني` |
|   ...its imlai keeps it | `أنبئوني` | quran-text hafs rasm_imlai 2:31 | `أنبئوني` | `انبيوني` | `نبيوني` |
| rub el hizb U+06DE (category So) | `۞ إِنَّ` | quran-tajweed uthmani-hafs.json 2:26 | `إن` | `ان` | `ن` |
| sajdah U+06E9 + small high madda U+06E4 | `وَٱلۡأٓصَالِ۩` | quran-tajweed uthmani-hafs.json 13:15 | `وٱلأصال` | `والاصال` | `ولصل` |
| waqf sign U+06D6 (category Mn) | `رَّبِّهِمۡۖ` | quran-tajweed uthmani-hafs.json 2:5 | `ربهم` | `ربهم` | `ربهم` |
| tatweel U+0640 (category Lm) | `أَنۢبِـُٔونِي` | quran-tajweed uthmani-hafs.json 2:31 | `أنبوني` | `انبوني` | `نبوني` |
