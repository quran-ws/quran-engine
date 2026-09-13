@AGENTS.md

## Claude Code only

- Run the `quranic-terminology` skill's `audit_terminology.py` before adding a Quranic
  name. `.terminology.json` is its config.
- Load the `ste-plain-writing` skill before writing any doc, comment or PR text, and run
  its linter on the result.
- Page data comes from the releases (`scripts/sync-test-data.sh`), never from a local
  machine.
