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
  hit testing, layout, styles, clock and display list, highlights, selection, memorisation,
  crop, dress, atlas, passage, names. The same feature sits at the same place in every
  language. `crates/qvp-ffi/include/qvp.h` is the list, so read it there when in doubt.
- Rust files: imports, constants, types, public impl, private impl, `mod tests` last.

## Per language

| language | file line | item docs | enforced by |
|---|---|---|---|
| Rust | `//!` | `///` | `#![warn(missing_docs)]` in `qvp-core` and `qvp-format`, errors in CI |
| C header | block comment at top | `/** … */` above each declaration | `check-parity.py` requires a block per symbol |
| JavaScript / TypeScript | `//` line | JSDoc `/** … */` | eslint `jsdoc/require-jsdoc` in the `standards` job (`scripts/check.sh docblocks`), with a baseline |
| Kotlin | `//` line | KDoc `/** … */` | detekt `UndocumentedPublicClass` and `UndocumentedPublicFunction` in the Android workflow, with a baseline |
| Dart | `//` line | `///` | `flutter analyze` in `.github/workflows/flutter.yml` runs the `public_member_api_docs` lint that the package's `analysis_options.yaml` turns on; `dart format --language-version=latest` in CI. Exempt: the generated `lib/src/bindings.dart` and the three backend files `qvp_backend.dart`, `qvp_backend_native.dart` and `qvp_backend_web.dart` in `lib/src/backend/`, because they only marshal C calls that `qvp.h` documents |
| Swift | `//` line | `///` | swiftlint `missing_docs` in the iOS workflow, with a baseline |
| shell / Python | purpose, inputs, outputs at the top | n/a | review |

## Formatting

`rustfmt.toml`, `.editorconfig` and each platform's formatter are the source of truth.
Reviews do not discuss whitespace. CI runs the formatters in check mode.

## How it is checked

CI runs the tools the table names. `check-parity.py` runs in the `standards` job. `cargo doc`
runs with missing docs as errors in the `engine` job (`scripts/check.sh docs`). The Flutter
workflow runs `flutter analyze`, which fails on a public Dart item with no doc block, and
`dart format --language-version=latest`. After a `pub get`, plain `dart format` reads the
package's minimum Dart version (3.6) and gives an older style. The flag keeps the newest
style. Where the table says review, no tool runs.

The three doc-block linters check what an app calls:

| linter | config | what it checks | baseline |
|---|---|---|---|
| eslint `jsdoc/require-jsdoc` | `scripts/lint/eslint.config.mjs` | `web/qvp.js`: every class, function and constant in the object that its `global.QVP = { … }` line assigns, and every method of an exported class whose name does not start with `_`. `web/lite*.mjs`: every exported function and class, and the public methods of an exported class. The React Native `src/`: every exported declaration, the public members of an exported class, and every member of `Qvp` | `scripts/lint/eslint-suppressions.json`: a count per file |
| detekt | `packages/android/detekt.yml` | every public class and function of `packages/android/qvp/src/main/kotlin`. A default companion object is left out | `packages/android/detekt-baseline.xml`: one entry per item |
| swiftlint `missing_docs` | `packages/ios/QvpKit/.swiftlint.yml` | every public declaration of `QvpKit/Sources` that swiftlint reports. swiftlint leaves out extensions, and the members of a type that inherits from another type | `packages/ios/QvpKit/swiftlint-baseline.json`: one entry per item |

A constructor needs no block of its own: the class block says how to make one. The baseline
lists the public items that had no doc block when the linter went in. A new item without one
fails CI. eslint also fails when a file has fewer items than its count, until you prune the
count. detekt and swiftlint keep a stale entry, so write their baseline again when you document
an item. Each config file names the command that writes its baseline. The baselines hold the
methods of the page and atlas classes in `web/qvp.js`, Kotlin and Swift, the members of the
React Native `Qvp` object, and Swift's `QvpPageCanvas`. A doc block on one of them, with the
baseline written again, makes the baseline shorter.
