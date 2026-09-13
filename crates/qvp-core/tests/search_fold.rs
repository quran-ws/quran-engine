//! The shared search-fold conformance vector.
//!
//! GENERATED from conformance/search-fold.json — do not edit by hand. The same
//! file is used by quran-text (JavaScript and Python) and quran-svg-elements,
//! so all three implementations are pinned to one specification.
//! See docs/SEARCH-FOLD.md.
//!
//! Every input is copied from a real data file in the quran-ws repositories.

use qvp_core::{loose_key, normalize_query, search_key, search_variants};

/// (input, search_key, match_fold, loose_key)
const VECTORS: &[(&str, &str, &str, &str)] = &[
    // dagger alif U+0670 is a MARK: stripped, never expanded
    //   source: quran-text hafs rasm_uthmani 1:1:3
    ("ٱلرَّحۡمَٰنِ", "ٱلرحمن", "الرحمن", "لرحمن"),
    //   ...the same word's imlai, the search-key source
    //   source: quran-text hafs rasm_imlai 1:1:3
    ("الرحمن", "الرحمن", "الرحمن", "لرحمن"),
    // dagger alif where imlai writes a FULL alif
    //   source: quran-text hafs rasm_uthmani 1:2:4
    ("ٱلۡعَٰلَمِينَ", "ٱلعلمين", "العلمين", "لعلمين"),
    //   ...its imlai
    //   source: quran-text hafs rasm_imlai 1:2:4
    ("العالمين", "العالمين", "العالمين", "لعلمين"),
    // alef wasla U+0671
    //   source: quran-text hafs rasm_uthmani
    ("ٱللَّهِ", "ٱلله", "الله", "لله"),
    // hamza above U+0623
    //   source: quran-text hafs rasm_uthmani
    ("أَنۡعَمۡتَ", "أنعمت", "انعمت", "نعمت"),
    // madda U+0622
    //   source: quran-text hafs rasm_uthmani
    ("ٱلضَّآلِّينَ", "ٱلضآلين", "الضالين", "لضلين"),
    // ta marbuta U+0629
    //   source: quran-text hafs rasm_uthmani
    ("ٱلصَّلَوٰةَ", "ٱلصلوة", "الصلوه", "لصلوه"),
    // waw hamza U+0624
    //   source: quran-text hafs rasm_uthmani
    ("يُؤۡمِنُونَ", "يؤمنون", "يومنون", "يومنون"),
    // yeh hamza U+0626
    //   source: quran-text hafs rasm_uthmani
    ("أُوْلَٰٓئِكَ", "أولئك", "اوليك", "وليك"),
    // bare hamza U+0621
    //   source: quran-text hafs rasm_uthmani
    ("سَوَآءٌ", "سوآء", "سواء", "سو"),
    // shadda + tanwin
    //   source: quran-text hafs rasm_uthmani
    ("رَغَدًا", "رغدا", "رغدا", "رغد"),
    // tatweel-borne hamza: stripping the uthmani LOSES the consonant
    //   source: quran-text hafs rasm_uthmani 2:31
    ("أَنۢبُِٔونِي", "أنبوني", "انبوني", "نبوني"),
    //   ...its imlai keeps it
    //   source: quran-text hafs rasm_imlai 2:31
    ("أنبئوني", "أنبئوني", "انبيوني", "نبيوني"),
    // rub el hizb U+06DE (category So)
    //   source: quran-tajweed uthmani-hafs.json 2:26
    ("۞ إِنَّ", "إن", "ان", "ن"),
    // sajdah U+06E9 + small high madda U+06E4
    //   source: quran-tajweed uthmani-hafs.json 13:15
    ("وَٱلۡأٓصَالِ۩", "وٱلأصال", "والاصال", "ولصل"),
    // waqf sign U+06D6 (category Mn)
    //   source: quran-tajweed uthmani-hafs.json 2:5
    ("رَّبِّهِمۡۖ", "ربهم", "ربهم", "ربهم"),
    // tatweel U+0640 (category Lm)
    //   source: quran-tajweed uthmani-hafs.json 2:31
    ("أَنۢبِـُٔونِي", "أنبوني", "انبوني", "نبوني"),
];

#[test]
fn conformance_vector() {
    for (input, want_key, want_fold, want_loose) in VECTORS {
        assert_eq!(&search_key(input), want_key, "search_key of {input:?}");
        assert_eq!(&normalize_query(input), want_fold, "match_fold of {input:?}");
        assert_eq!(&loose_key(input), want_loose, "loose_key of {input:?}");
    }
}

#[test]
fn dagger_alif_is_a_mark_not_an_alif() {
    // The bug this spec exists to settle: quran-text used to expand U+0670 into
    // a full alif, so two blocks disagreed about one query.
    assert_eq!(
        normalize_query(
            "\u{0671}\u{0644}\u{0631}\u{0651}\u{064E}\u{062D}\u{06E1}\u{0645}\u{064E}\u{0670}\u{0646}\u{0650}"
        ),
        "\u{0627}\u{0644}\u{0631}\u{062D}\u{0645}\u{0646}"
    );
    assert_eq!(
        loose_key("\u{0627}\u{0644}\u{0631}\u{062D}\u{0645}\u{0627}\u{0646}"),
        loose_key("\u{0627}\u{0644}\u{0631}\u{062D}\u{0645}\u{0646}")
    );
}

#[test]
fn variants_cover_both_spellings() {
    // مَٰلِكِ — uthmani with a dagger alif; modern spelling writes مالك.
    let v = search_variants("\u{0645}\u{064E}\u{0670}\u{0644}\u{0650}\u{0643}\u{0650}");
    assert!(v.contains(&"\u{0645}\u{0644}\u{0643}".to_string()), "stripped spelling");
    assert!(v.contains(&"\u{0645}\u{0627}\u{0644}\u{0643}".to_string()), "imlai spelling");
}
