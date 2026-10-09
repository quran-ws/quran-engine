#!/usr/bin/env node
/** Audit the complete QCF V1 QVP corpus for its pinned semantic contract. */

import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { decodeGeometry } from "../web/lite.mjs";

const KIND_MARK = 1;
const FAMILY_NONE = 0;
const FAMILY_WAQF = 4;
const FAMILY_STANDALONE = 6;
const DECO_DIVISION = 3;
const DECO_SAJDAH = 4;
const MARK_HIZB = 28;
const MARK_SAJDAH = 30;
const FLAG_JUZ = 1;
const FLAG_HIZB = 2;
const FLAG_RUBU_AL_HIZB = 4;
const FLAG_NISF = 8;
const PAGE_FILE = /^[0-9]{3}\.qvp$/;
const DIVISION_TEXT = /^juz=(\d+);hizb=(\d+);rubu_al_hizb=(\d+);nisf=(\d+)$/;

const WAQF_MARKS = new Map([
  [21, "waqf_jaiz_mustawi_al_tarafayn"],
  [22, "waqf_jaiz_waqf_awla"],
  [23, "waqf_jaiz_wasl_awla"],
  [24, "waqf_lazim"],
  [25, "waqf_al_muanaqah"],
]);

const UNPRINTED_DIVISIONS = Object.freeze([
  1, 31, 43, 61, 71, 75, 105, 107, 113, 125, 129, 133, 137, 140, 144, 147,
  151, 159, 162, 167, 187, 201, 206, 213, 214, 217, 221, 223, 224, 225, 226,
  227, 229, 231, 233, 234, 235, 236, 237, 238, 239,
]);

const SAJDAH_SITES = Object.freeze([
  "176:7:206",
  "251:13:15",
  "272:16:50",
  "293:17:109",
  "309:19:58",
  "334:22:18",
  "341:22:77",
  "365:25:60",
  "379:27:26",
  "416:32:15",
  "454:38:24",
  "480:41:38",
  "528:53:62",
  "589:84:21",
  "597:96:19",
]);

export const EXPECTED_QCF_V1_SEMANTICS = Object.freeze({
  pages: 604,
  waqf: Object.freeze({
    paths: 4221,
    byMark: Object.freeze({
      waqf_al_muanaqah: 6,
      waqf_jaiz_mustawi_al_tarafayn: 2066,
      waqf_jaiz_waqf_awla: 511,
      waqf_jaiz_wasl_awla: 1617,
      waqf_lazim: 21,
    }),
  }),
  divisions: Object.freeze({
    boundaries: 240,
    printed: 199,
    unprinted: UNPRINTED_DIVISIONS,
    flags: Object.freeze({ juz: 30, hizb: 60, rubu_al_hizb: 240, nisf: 60 }),
  }),
  sajdahSites: SAJDAH_SITES,
});

export function createSemanticAudit() {
  return {
    pages: 0,
    waqfPaths: 0,
    waqfByMark: Object.fromEntries(
      [...WAQF_MARKS.values()].sort().map((name) => [name, 0]),
    ),
    boundarySites: {},
    divisionSites: {},
    divisionFlags: { juz: 0, hizb: 0, rubu_al_hizb: 0, nisf: 0 },
    divisionPathMarks: 0,
    sajdahSites: [],
    sajdahPathMarks: 0,
  };
}

function site(page, surah, ayah) {
  return `${page}:${surah}:${ayah}`;
}

function expectedDivisionFlags(rubuAlHizb) {
  let flags = FLAG_RUBU_AL_HIZB;
  if ((rubuAlHizb - 3) % 4 === 0) flags |= FLAG_NISF;
  if ((rubuAlHizb - 1) % 4 === 0) flags |= FLAG_HIZB;
  if ((rubuAlHizb - 1) % 8 === 0) flags |= FLAG_JUZ;
  return flags;
}

function onePath(geometry, decoration, source) {
  if (decoration.nPaths !== 1) {
    throw new Error(`${source}: semantic decoration does not own exactly one path`);
  }
  const index = decoration.firstPath;
  const path = geometry.paths[index];
  if (!path) throw new Error(`${source}: semantic decoration path is missing`);
  return [index, path];
}

export function auditSemanticGeometry(
  geometry,
  page = geometry?.number,
  source = "QVP page",
  audit = createSemanticAudit(),
) {
  if (
    !geometry ||
    !Array.isArray(geometry.paths) ||
    !Array.isArray(geometry.ayahs) ||
    !Array.isArray(geometry.decorations)
  ) {
    throw new TypeError(`${source}: decoded geometry is incomplete`);
  }
  if (!Number.isInteger(page) || page < 1) throw new TypeError(`${source}: invalid page`);
  audit.pages += 1;

  const ownedDivisionPaths = new Set();
  const ownedSajdahPaths = new Set();
  for (let index = 0; index < geometry.paths.length; index += 1) {
    const path = geometry.paths[index];
    const name = WAQF_MARKS.get(path.mark);
    const hasWaqfIdentity = name !== undefined || path.family === FAMILY_WAQF;
    if (hasWaqfIdentity) {
      if (
        path.kind !== KIND_MARK ||
        path.family !== FAMILY_WAQF ||
        name === undefined
      ) {
        throw new Error(
          `${source}: path ${index} has incoherent waqf semantics ` +
            `(kind=${path.kind}, mark=${path.mark}, family=${path.family})`,
        );
      }
      audit.waqfPaths += 1;
      audit.waqfByMark[name] += 1;
    }
  }

  for (const ayah of geometry.ayahs) {
    const semanticFlags = ayah.flags & 15;
    const rubuAlHizb = ayah.rubuAlHizb;
    if (!rubuAlHizb) {
      if (semanticFlags) throw new Error(`${source}: division flags have no rubu-al-hizb value`);
      continue;
    }
    if (
      !Number.isInteger(rubuAlHizb) ||
      rubuAlHizb < 1 ||
      rubuAlHizb > 240 ||
      ayah.fragment !== 1 ||
      semanticFlags !== expectedDivisionFlags(rubuAlHizb)
    ) {
      throw new Error(`${source}: incoherent rubu-al-hizb ${rubuAlHizb} boundary`);
    }
    if (Object.hasOwn(audit.boundarySites, rubuAlHizb)) {
      throw new Error(`${source}: duplicate rubu-al-hizb ${rubuAlHizb} boundary`);
    }
    audit.boundarySites[rubuAlHizb] = site(page, ayah.surah, ayah.ayah);
    audit.divisionFlags.rubu_al_hizb += 1;
    if (semanticFlags & FLAG_JUZ) audit.divisionFlags.juz += 1;
    if (semanticFlags & FLAG_HIZB) audit.divisionFlags.hizb += 1;
    if (semanticFlags & FLAG_NISF) audit.divisionFlags.nisf += 1;
  }

  for (const decoration of geometry.decorations) {
    if (decoration.decoration === DECO_DIVISION) {
      const match = DIVISION_TEXT.exec(decoration.text);
      if (!match || decoration.lineIndex < 0) {
        throw new Error(`${source}: invalid division metadata`);
      }
      const [juz, hizb, rubuAlHizb, nisf] = match.slice(1).map(Number);
      if (
        rubuAlHizb < 1 ||
        rubuAlHizb > 240 ||
        juz !== Math.floor((rubuAlHizb - 1) / 8) + 1 ||
        hizb !== Math.floor((rubuAlHizb - 1) / 4) + 1 ||
        nisf !== Math.floor(((rubuAlHizb - 1) % 4) / 2) + 1 ||
        Object.hasOwn(audit.divisionSites, rubuAlHizb)
      ) {
        throw new Error(`${source}: incoherent printed rubu-al-hizb ${rubuAlHizb}`);
      }
      const [index, path] = onePath(geometry, decoration, source);
      if (
        path.kind !== KIND_MARK ||
        path.mark !== MARK_HIZB ||
        path.family !== FAMILY_NONE
      ) {
        throw new Error(`${source}: incoherent printed division path`);
      }
      ownedDivisionPaths.add(index);
      audit.divisionPathMarks += 1;
      audit.divisionSites[rubuAlHizb] = site(page, decoration.surah, decoration.ayah);
    } else if (decoration.decoration === DECO_SAJDAH) {
      if (decoration.text !== "" || decoration.lineIndex < 0) {
        throw new Error(`${source}: invalid sajdah metadata`);
      }
      const [index, path] = onePath(geometry, decoration, source);
      if (
        path.kind !== KIND_MARK ||
        path.mark !== MARK_SAJDAH ||
        path.family !== FAMILY_STANDALONE
      ) {
        throw new Error(`${source}: incoherent sajdah path`);
      }
      ownedSajdahPaths.add(index);
      audit.sajdahPathMarks += 1;
      audit.sajdahSites.push(site(page, decoration.surah, decoration.ayah));
    }
  }

  for (let index = 0; index < geometry.paths.length; index += 1) {
    const mark = geometry.paths[index].mark;
    if (mark === MARK_HIZB && !ownedDivisionPaths.has(index)) {
      throw new Error(`${source}: hizb path is outside a division decoration`);
    }
    if (mark === MARK_SAJDAH && !ownedSajdahPaths.has(index)) {
      throw new Error(`${source}: sajdah path is outside a sajdah decoration`);
    }
  }
  return audit;
}

function sameValues(actual, expected) {
  return JSON.stringify(actual) === JSON.stringify(expected);
}

export function verifyQcfV1SemanticAudit(
  audit,
  expected = EXPECTED_QCF_V1_SEMANTICS,
) {
  if (audit.pages !== expected.pages) {
    throw new Error(`QCF V1 page inventory differs: ${audit.pages}`);
  }
  const waqfNames = Object.keys(expected.waqf.byMark).sort();
  if (
    audit.waqfPaths !== expected.waqf.paths ||
    !sameValues(Object.keys(audit.waqfByMark).sort(), waqfNames) ||
    waqfNames.some(
      (name) => audit.waqfByMark[name] !== expected.waqf.byMark[name],
    )
  ) {
    throw new Error(`QCF V1 waqf inventory differs`);
  }

  const boundaryValues = Object.keys(audit.boundarySites)
    .map(Number)
    .sort((a, b) => a - b);
  const expectedBoundaries = Array.from(
    { length: expected.divisions.boundaries },
    (_, index) => index + 1,
  );
  const printedValues = Object.keys(audit.divisionSites)
    .map(Number)
    .sort((a, b) => a - b);
  const unprintedValues = expectedBoundaries.filter(
    (value) => !Object.hasOwn(audit.divisionSites, value),
  );
  if (
    !sameValues(boundaryValues, expectedBoundaries) ||
    printedValues.length !== expected.divisions.printed ||
    !sameValues(unprintedValues, expected.divisions.unprinted) ||
    !sameValues(audit.divisionFlags, expected.divisions.flags) ||
    audit.divisionPathMarks !== expected.divisions.printed ||
    printedValues.some(
      (value) => audit.divisionSites[value] !== audit.boundarySites[value],
    )
  ) {
    throw new Error(`QCF V1 division inventory differs`);
  }

  const sajdahSites = [...audit.sajdahSites].sort();
  const expectedSajdahSites = [...expected.sajdahSites].sort();
  if (
    audit.sajdahPathMarks !== expectedSajdahSites.length ||
    !sameValues(sajdahSites, expectedSajdahSites)
  ) {
    throw new Error(`QCF V1 sajdah inventory differs`);
  }

  return {
    schema: "quran-engine/qcf-v1-semantic-audit",
    schema_version: 1,
    edition: "hafs-qcf-v1",
    pages: audit.pages,
    waqf: { paths: audit.waqfPaths, by_mark: audit.waqfByMark },
    divisions: {
      boundaries: boundaryValues.length,
      printed: printedValues.length,
      unprinted: unprintedValues,
      flags: audit.divisionFlags,
    },
    sajdah: { marks: audit.sajdahPathMarks, sites: sajdahSites },
  };
}

export async function auditQcfV1SemanticDirectory(directory) {
  const root = resolve(directory);
  const files = (await readdir(root))
    .filter((name) => PAGE_FILE.test(name))
    .sort();
  const expectedFiles = Array.from(
    { length: 604 },
    (_, index) => `${String(index + 1).padStart(3, "0")}.qvp`,
  );
  if (!sameValues(files, expectedFiles)) {
    throw new Error(`QCF V1 page file inventory differs in ${root}`);
  }

  const audit = createSemanticAudit();
  for (const name of files) {
    const page = Number(name.slice(0, 3));
    const geometry = decodeGeometry(await readFile(resolve(root, name)));
    if (geometry.number !== page) {
      throw new Error(`${name}: decoded page number is ${geometry.number}`);
    }
    auditSemanticGeometry(geometry, page, name, audit);
  }
  return verifyQcfV1SemanticAudit(audit);
}

const invoked =
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
if (invoked) {
  if (process.argv.length !== 3) {
    console.error("usage: node scripts/audit-qcf-v1-semantics.mjs <qvp-directory>");
    process.exit(2);
  }
  try {
    console.log(
      JSON.stringify(await auditQcfV1SemanticDirectory(process.argv[2]), null, 2),
    );
  } catch (error) {
    console.error(`error: ${error.message}`);
    process.exit(1);
  }
}
