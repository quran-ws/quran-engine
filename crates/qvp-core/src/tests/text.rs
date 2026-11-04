//! Text, search, selection and citation.
use super::*;

#[test]
fn text_search_selection_citation() {
    let p = page();
    assert_eq!(p.text_of(&[0, 1, 2], Form::RasmUthmani, " ", "\n"), "ذَٰلِكَ ٱلْكِتَٰبُ\nلَا");
    assert_eq!(p.text_of(&p.target_words(&Target::Page), Form::Search, " ", " / "), "ذلك الكتاب / لا");
    let m = p.search("الكتاب", &SearchOptions::default());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].word, 1);
    assert!(p.search("كتاب", &SearchOptions { mode: SearchMode::Exact, ..Default::default() }).is_empty());
    assert_eq!(
        p.search("الكتب", &SearchOptions { loose: false, ..Default::default() }).len(),
        0,
        "strict pass: different letters"
    );
    assert_eq!(p.search("الكتب", &SearchOptions::default()).len(), 1, "loose pass drops bare alef");
    assert_eq!(p.search("ذَلِكَ", &SearchOptions::default())[0].word, 0, "query is normalised");
    let s = Selection { anchor: Some(2), focus: Some(0) };
    assert_eq!(s.words(), vec![0, 1, 2]);
    assert_eq!(p.citation(&s.words()), "1:1-2");
    assert_eq!(p.citation(&[1]), "1:1");
    assert_eq!(p.divisions().iter().map(|d| (d.kind, d.n)).collect::<Vec<_>>(), vec![(0, 1), (1, 1), (3, 1)]);
    assert_eq!(p.ayah_keys(), vec![(1, 1), (1, 2)]);
    assert_eq!(p.recite_map(1, 1, 2), Some(vec![0, 1]));
    assert_eq!(p.recite_map(1, 1, 3), None, "count mismatch → follow whole");
}
