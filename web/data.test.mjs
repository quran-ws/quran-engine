import assert from 'node:assert/strict'
import { qvpEditions, qvpSource, verifyQvpManifest } from './data.mjs'

const sha = value => value.toString(16).padStart(64, '0')

function manifest(source) {
  const files = []
  const add = name => files.push({ name, bytes: 1, sha256: sha(files.length + 1) })
  for (let page = 1; page <= 604; page++) {
    const name = String(page).padStart(3, '0')
    add(`${name}.qvp`)
    add(`${name}.words.json`)
  }
  add('atlas.qva')
  add('atlas.json')
  add('VERSION.json')
  for (let surah = 1; surah <= 114; surah++) {
    const name = String(surah).padStart(3, '0')
    add(`surah-names/qvp/${name}.qvp`)
    add(`surah-names/svg/${name}.svg`)
  }
  for (const name of ['surah-names/qvp/all.qvp', 'surah-names/svg/all.svg', 'surah-names/surah-names.woff2', 'surah-names/surah-names.css', 'surah-names/map.json']) add(name)
  return {
    version: source.version,
    base: `${source.baseUrl}/`,
    encoding: 'identity',
    release: {
      ...(source.releaseSchema === null ? {} : { schema: source.releaseSchema, schema_version: source.releaseSchemaVersion }),
      package: source.packageName,
      version: source.version.slice(1),
      edition: source.edition,
      print_year_hijri: source.printYearHijri,
      format_version: 1,
      ...(source.semantics === null ? {} : {
        source: {
          summary_schema_version: source.semantics.sourceSchemaVersion,
          qualification: source.semantics.qualification,
          hq_words: source.semantics.hqWords,
          fallback_words: source.semantics.fallbackWords,
          pages_with_hq: source.semantics.pagesWithHq,
          summary_sha256: source.semantics.summarySha256,
          pages_sha256: source.semantics.pagesSha256,
          qualified_geometry_pages_sha256: source.semantics.qualifiedGeometryPagesSha256,
          source_candidate_tree_sha256: source.semantics.candidateTreeSha256,
          source_manifest_sha256: source.semantics.sourceManifestSha256,
          source_records_sha256: source.semantics.sourceRecordsSha256,
          optical_calibration_sha256: source.semantics.opticalCalibrationSha256,
          source_exclusions: { ...source.semantics.sourceExclusions },
          waqf: {
            separate_source_glyphs: source.semantics.waqfPaths,
            fused_source_glyphs: source.semantics.fusedWaqf,
          },
          base_path_geometry: {
            qualification: 'base-order-preserved',
            base_paths: source.semantics.basePaths,
            emitted_paths: source.semantics.emittedPaths,
            restored_paths: source.semantics.restoredPaths,
          },
          division_sajdah: {
            qualification: 'source-owned',
            division_starts: source.semantics.divisionStarts,
            printed_rubu_al_hizb: source.semantics.printedDivisions,
            unprinted_rubu_al_hizb: source.semantics.unprintedDivisions,
            mapped_sajdah_source_glyphs: source.semantics.mappedSajdah,
            restored_sajdah_source_glyphs: source.semantics.restoredSajdah,
            sajdah_marks: source.semantics.sajdahMarks,
          },
        },
      }),
      qvp: { files: 1443, pages: 604, word_sidecars: 604, logical_words: 77432, surah_names: 114 },
      payload_files: Object.fromEntries(
        files.filter(file => file.name !== 'VERSION.json').map(file => [file.name, file.sha256]),
      ),
    },
    bundle: {
      name: source.bundleName,
      bytes: 1,
      sha256: sha(9999),
      decoded: { bytes: 2, sha256: sha(9998) },
    },
    files,
  }
}

assert.deepEqual(Object.keys(qvpEditions), ['hafs-kfgqpc', 'hafs-qcf-v1-1405h'])
assert.ok(Object.isFrozen(qvpEditions) && Object.values(qvpEditions).every(Object.isFrozen))

const standard = qvpSource('hafs-kfgqpc', '0.4.0')
assert.equal(standard.baseUrl, 'https://cdn.quran.ws/qvp/v0.4.0')
assert.equal(standard.latestUrl, 'https://cdn.quran.ws/qvp/latest.json')
assert.equal(standard.pageUrl(42), 'https://cdn.quran.ws/qvp/v0.4.0/042.qvp')
assert.equal(standard.wordSidecarUrl(42), 'https://cdn.quran.ws/qvp/v0.4.0/042.words.json')
assert.equal(standard.surahNameUrl(1), 'https://cdn.quran.ws/qvp/v0.4.0/surah-names/qvp/001.qvp')
assert.equal(standard.surahNameUrl(114, 'svg'), 'https://cdn.quran.ws/qvp/v0.4.0/surah-names/svg/114.svg')
assert.equal(standard.surahNamesUrl('svg'), 'https://cdn.quran.ws/qvp/v0.4.0/surah-names/svg/all.svg')

const qcf = qvpSource('hafs-qcf-v1-1405h', 'v0.1.0', { origin: 'https://static.example.test/' })
assert.equal(qcf.edition, 'hafs-qcf-v1')
assert.equal(qcf.printYearHijri, 1405)
assert.equal(qcf.releaseSchema, 'quran-engine/edition-data-release')
assert.equal(qcf.releaseSchemaVersion, 7)
assert.equal(qcf.semantics.hqWords, 60739)
assert.equal(qcf.semantics.fallbackWords, 16693)
assert.equal(qcf.semantics.pagesWithHq, 602)
assert.equal(qcf.semantics.qualification, 'source-qualified')
assert.equal(qcf.semantics.sourceExclusions['unsupported-source'], 5043)
assert.equal(qcf.semantics.sourceExclusions['reviewed-fallback'], 62)
assert.equal(qcf.semantics.opticalCalibrationSha256, '6323f6f8f77b42150e3b666b74f4789d01b41096ffd09718c4e552abdfaf3059')
assert.ok(Object.isFrozen(qcf.semantics.sourceExclusions))
assert.equal(qcf.semantics.waqfPaths, 4221)
assert.equal(qcf.semantics.fusedWaqf, 51)
assert.equal(qcf.semantics.printedDivisions, 199)
assert.equal(qcf.semantics.sajdahMarks, 15)
assert.ok(Object.isFrozen(qcf.semantics))
assert.equal(qcf.baseUrl, 'https://static.example.test/qvp/hafs-qcf-v1-1405h/v0.1.0')
assert.equal(qcf.latestUrl, 'https://static.example.test/qvp/hafs-qcf-v1-1405h/latest.json')
assert.equal(qcf.bundleUrl, `${qcf.baseUrl}/hafs-qcf-v1-1405h.tar.br`)
assert.equal(qcf.atlasUrl, `${qcf.baseUrl}/atlas.qva`)
assert.equal(qcf.atlasMetadataUrl, `${qcf.baseUrl}/atlas.json`)
assert.equal(qcf.surahNameFontUrl, `${qcf.baseUrl}/surah-names/surah-names.woff2`)
assert.ok(Object.isFrozen(qcf))

for (const args of [['unknown', '1.0.0'], ['hafs-kfgqpc', '1'], ['hafs-kfgqpc', '01.2.3'], ['hafs-kfgqpc', 'data-v1.0.0']])
  assert.throws(() => qvpSource(...args), /QVP data/)
for (const page of [0, 605, 1.5, '1']) assert.throws(() => qcf.pageUrl(page), /Quran page/)
for (const surah of [0, 115]) assert.throws(() => qcf.surahNameUrl(surah), /Surah/)
assert.throws(() => qcf.surahNameUrl(1, 'png'), /format/)
for (const invalidOrigin of ['', 'javascript:alert(1)', 'https://user@example.com', 'https://example.com/?x=1'])
  assert.throws(() => qvpSource('hafs-kfgqpc', '1.0.0', { origin: invalidOrigin }), /origin/)

const valid = manifest(qcf)
assert.equal(verifyQvpManifest(qcf, valid), valid)
for (const change of [
  value => { value.version = 'v9.9.9' },
  value => { value.base = `${standard.baseUrl}/` },
  value => { value.release.package = standard.packageName },
  value => { value.release.schema_version = 6 },
  value => { value.release.schema = 'other' },
  value => { value.release.source.waqf.separate_source_glyphs = 4220 },
  value => { value.release.source.hq_words = 64 },
  value => { value.release.source.source_exclusions['typed-owner'] = 4220 },
  value => { value.release.source.source_records_sha256 = sha(1234) },
  value => { value.release.source.optical_calibration_sha256 = sha(1235) },
  value => { value.release.source.base_path_geometry.restored_paths = 0 },
  value => { value.release.source.division_sajdah.sajdah_marks = 14 },
  value => { value.release.edition = standard.edition },
  value => { value.release.print_year_hijri = 1441 },
  value => { value.release.qvp.pages = 603 },
  value => { value.bundle.name = standard.bundleName },
  value => { value.bundle.decoded.sha256 = 'bad' },
  value => { value.release.qvp.files = 1442 },
  value => { value.release.qvp.logical_words = 1 },
  value => { value.release.payload_files['001.qvp'] = sha(7777) },
  value => { delete value.release.payload_files['002.qvp'] },
  value => { value.files.pop() },
  value => { value.files[0].sha256 = 'bad' },
  value => { value.files.push({ ...value.files[0] }) },
  value => { value.files[0].name = '../001.qvp' },
  value => { value.files.push({ name: 'foreign.qvp', bytes: 1, sha256: sha(8888) }) },
]) {
  const changed = structuredClone(valid)
  change(changed)
  assert.throws(() => verifyQvpManifest(qcf, changed), /QVP|Missing|Incomplete|Duplicate|Invalid/)
}
assert.throws(() => verifyQvpManifest({}, valid), /source/)

// Older default releases use `name` and top-level counts rather than the edition-package shape.
const legacy = manifest(standard)
legacy.files.push({ name: 'NOTICE.txt', bytes: 0, sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855' })
legacy.release = {
  name: standard.packageName,
  version: '0.4.0',
  format_version: 1,
  pages: 604,
  surah_names: 114,
}
assert.equal(verifyQvpManifest(standard, legacy), legacy)

console.log('ok  QVP editions, immutable URLs, manifest identity and complete inventories')
