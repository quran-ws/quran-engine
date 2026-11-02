## What changes

<!-- One paragraph. What a reader of the changelog needs to know. -->

## Tier

<!-- Keep the one that applies and delete the rest. See docs/standards/API-DESIGN.md. -->

- Core behaviour: changed in Rust once; every platform receives it with the next native build.
- Engine API: a C symbol. Bound in all five wrappers under the parity table's name, or declared in `docs/API-PARITY.md` as a gap with an issue number.
- Lite passage layout: the no-Wasm port in `web/lite-passage.mjs`. It follows `crates/qvp-core/src/passage.rs` and replays `conformance/scenarios/passage.json`.
- Platform convenience: one platform's idiom, documented in that package's README. It computes nothing the engine can compute.
- None of these: tooling, CI, tests or documents. Say which.

## Checklist

- [ ] `scripts/check.sh` passes locally.
- [ ] Tests ship with the change (`docs/standards/TESTING.md`).
- [ ] A line under Unreleased in `CHANGELOG.md`.
- [ ] Names follow `docs/standards/NAMING.md`; new Quranic terms passed the terminology audit.
- [ ] For an engine API change: `docs/API.md` updated and `scripts/check-parity.py` run.
