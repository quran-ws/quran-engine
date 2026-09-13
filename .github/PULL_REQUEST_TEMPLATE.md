## What changes

<!-- One paragraph. What a reader of the changelog needs to know. -->

## Tier

<!-- Delete the two that do not apply. See docs/standards/API-DESIGN.md. -->

- Core behaviour: changed in Rust once; every platform receives it with the next native build.
- Engine API: a C symbol. Bound in all five wrappers under the parity table's name, or declared in `docs/API-PARITY.md` as a gap with an issue number.
- Platform convenience: one platform's idiom, documented in that package's README. It computes nothing the engine can compute.

## Checklist

- [ ] `scripts/check.sh` passes locally.
- [ ] Tests ship with the change (`docs/standards/TESTING.md`).
- [ ] A line under Unreleased in `CHANGELOG.md`.
- [ ] Names follow `docs/standards/NAMING.md`; new Quranic terms passed the terminology audit.
- [ ] For an engine API change: `docs/API.md` updated and `scripts/check-parity.py` run.
