# Code style: comments, doc blocks, file layout

One standard for Rust, C, JavaScript, Kotlin, Dart and Swift.

## Doc blocks

- Every public item has one.
- The first line is one sentence: a third-person verb from the vocabulary in `NAMING.md`,
  written from the caller's side, ending with a period. "Get the word under a point, in
  page units." "Add a highlight and return its handle."
- Nothing restates a type. The language's types carry that.
- Details only for what the signature cannot say: units, ownership, lifetime, what a
  sentinel return means. The C header lists out-parameters and error returns in that order.
- One summary sentence per engine function. It is canonical in the header's block and
  repeated in `docs/API.md`. A wrapper must have a doc block on every binding and may phrase
  it in its own idiom. It does not paste the string.

## Inline comments

- They explain why, never what. A full sentence, sentence case, a period.
- A narrative comment before a non-obvious block is welcome: "The printed spacing is the
  floor, so a fill that would need to tighten leaves the page as printed."
- No commented-out code. A TODO carries an issue number or does not exist.

## Files

- A file opens with one line saying what it is for.
- No licence or copyright banners. The composite `LICENSE` maps licences by directory.
- Every wrapper file follows the header's section order: page, metadata, text and search,
  hit testing, layout, styles, colours, highlights, selection, mask, reveal, crop, atlas,
  names. The same feature sits at the same place in every language.
- Rust files: imports, constants, types, public impl, private impl, `mod tests` last.

## Per language

| language | file line | item docs | enforced by |
|---|---|---|---|
| Rust | `//!` | `///` | `#![warn(missing_docs)]` in `qvp-core` and `qvp-format`, errors in CI |
| C header | block comment at top | `/** … */` above each declaration | `check-parity.py` requires a block per symbol |
| JavaScript / TypeScript | `//` line | JSDoc `/** … */` | eslint `jsdoc/require-jsdoc` on exports |
| Kotlin | `//` line | KDoc `/** … */` | detekt `UndocumentedPublicFunction` |
| Dart | `//` line | `///` | `public_member_api_docs` lint |
| Swift | `//` line | `///` | swiftlint `missing_docs` |
| shell / Python | purpose, inputs, outputs at the top | n/a | review |

## Formatting

`rustfmt.toml`, `.editorconfig` and each platform's formatter are the source of truth.
Reviews do not discuss whitespace. CI runs the formatters in check mode.

## How it is checked

The `standards` job in CI runs every "enforced by" tool above and `cargo doc` with missing
docs as errors.
