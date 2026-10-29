import assert from "node:assert/strict";

import {
  auditSemanticGeometry,
  createSemanticAudit,
  verifyQcfV1SemanticAudit,
} from "../scripts/audit-qcf-v1-semantics.mjs";

const expected = {
  pages: 1,
  waqf: {
    paths: 5,
    byMark: {
      waqf_al_muanaqah: 1,
      waqf_jaiz_mustawi_al_tarafayn: 1,
      waqf_jaiz_waqf_awla: 1,
      waqf_jaiz_wasl_awla: 1,
      waqf_lazim: 1,
    },
  },
  divisions: {
    boundaries: 1,
    printed: 1,
    unprinted: [],
    flags: { juz: 1, hizb: 1, rubu_al_hizb: 1, nisf: 0 },
  },
  sajdahSites: ["1:2:3"],
};

function geometry() {
  return {
    number: 1,
    paths: [
      { kind: 1, mark: 21, family: 4 },
      { kind: 1, mark: 22, family: 4 },
      { kind: 1, mark: 23, family: 4 },
      { kind: 1, mark: 24, family: 4 },
      { kind: 1, mark: 25, family: 4 },
      { kind: 1, mark: 28, family: 0 },
      { kind: 1, mark: 30, family: 6 },
    ],
    ayahs: [
      { surah: 2, ayah: 3, fragment: 1, flags: 7, rubuAlHizb: 1 },
    ],
    decorations: [
      {
        decoration: 3,
        surah: 2,
        ayah: 3,
        lineIndex: 0,
        text: "juz=1;hizb=1;rubu_al_hizb=1;nisf=1",
        firstPath: 5,
        nPaths: 1,
      },
      {
        decoration: 4,
        surah: 2,
        ayah: 3,
        lineIndex: 0,
        text: "",
        firstPath: 6,
        nPaths: 1,
      },
    ],
  };
}

const audit = auditSemanticGeometry(geometry(), 1, "synthetic");
const report = verifyQcfV1SemanticAudit(audit, expected);
assert.equal(report.waqf.paths, 5);
assert.equal(report.divisions.printed, 1);
assert.deepEqual(report.sajdah.sites, ["1:2:3"]);

for (const mutate of [
  (value) => { value.paths[0].family = 0; },
  (value) => { value.ayahs[0].flags = 15; },
  (value) => { value.paths[5].mark = 0; },
  (value) => { value.paths[6].family = 0; },
  (value) => { value.decorations[0].text = "bad"; },
]) {
  const value = geometry();
  mutate(value);
  assert.throws(
    () => auditSemanticGeometry(value, 1, "synthetic", createSemanticAudit()),
    /incoherent|invalid/,
  );
}

assert.throws(
  () => verifyQcfV1SemanticAudit(createSemanticAudit(), expected),
  /page inventory differs/,
);
const wrong = structuredClone(audit);
wrong.sajdahSites = [];
assert.throws(
  () => verifyQcfV1SemanticAudit(wrong, expected),
  /sajdah inventory differs/,
);
assert.throws(() => auditSemanticGeometry({}, 1, "synthetic"), TypeError);

console.log("ok  QCF V1 semantic audit rejects waqf, division and sajdah drift");
