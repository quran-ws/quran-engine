// Immutable QVP release locations. This entry point performs no I/O.

const CDN_ORIGIN = 'https://cdn.quran.ws'
const SHA256 = /^[0-9a-f]{64}$/
const VERSION = /^v?(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/
const SOURCE = Symbol('qvp-source')
const QCF_V1_SOURCE_EXCLUSIONS = Object.freeze({
  'aspect-ratio': 456,
  'candidate-missing': 52,
  'excluded-page': 62,
  'excluded-word': 1,
  'fit-audit': 39,
  'reported-centre': 1198,
  'reported-size': 1490,
  'review-marked': 759,
  'reviewed-fallback': 62,
  'scale-drift': 1070,
  'scan-iou': 69,
  'semantic-owner': 214,
  'text-differs': 1957,
  'typed-owner': 4221,
  'unsupported-source': 5043,
})
const QCF_V1_SEMANTICS = Object.freeze({
  sourceSchemaVersion: 10,
  qualification: 'source-qualified',
  hqWords: 60739,
  fallbackWords: 16693,
  pagesWithHq: 602,
  summarySha256: 'b30ecbb475ea80027812822fc5c5e93f8fc51d6c5dbd116d6fb85a0f64e19858',
  pagesSha256: 'a394d850efb83eab405f540c885033348ea2fde460a94f5ea0bbf671cb0ce55c',
  qualifiedGeometryPagesSha256: '18f32701b14e588610a7c2bd73495644eb9e36ea9b0fb87a46ed8c316dd09d8e',
  candidateTreeSha256: 'bba389e5fe0a533acd8c0a00698a4d8671b697ff362754fb19d884bd51a46636',
  sourceManifestSha256: 'c2f16ad2518ad69a618a70cb3858a03f56bb52f4d9391dfc15a04dc3a9e388d0',
  sourceRecordsSha256: '0904180d75f02f9229330bb0407f2b047a172df9dcd44c7aa3d6b815c4e3180d',
  opticalCalibrationSha256: '6323f6f8f77b42150e3b666b74f4789d01b41096ffd09718c4e552abdfaf3059',
  sourceExclusions: QCF_V1_SOURCE_EXCLUSIONS,
  waqfPaths: 4221,
  fusedWaqf: 51,
  basePaths: 88472,
  emittedPaths: 88473,
  restoredPaths: 1,
  divisionStarts: 240,
  printedDivisions: 199,
  unprintedDivisions: 41,
  mappedSajdah: 14,
  restoredSajdah: 1,
  sajdahMarks: 15,
})

function edition(key, edition, printYearHijri, family, packageName, bundleName, releaseSchema = null, releaseSchemaVersion = null, semantics = null) {
  return Object.freeze({ key, edition, printYearHijri, family, packageName, bundleName, releaseSchema, releaseSchemaVersion, semantics, pageCount: 604, surahCount: 114, wordCount: 77432, formatVersion: 1 })
}

/** QVP data editions available through the release/CDN contract. */
export const qvpEditions = Object.freeze({
  'hafs-kfgqpc': edition('hafs-kfgqpc', 'hafs-kfgqpc', 1441, 'qvp', 'quran-engine-pages-hafs-kfgqpc', 'hafs-kfgqpc.tar.br'),
  'hafs-qcf-v1-1405h': edition('hafs-qcf-v1-1405h', 'hafs-qcf-v1', 1405, 'qvp/hafs-qcf-v1-1405h', 'quran-engine-pages-hafs-qcf-v1-1405h', 'hafs-qcf-v1-1405h.tar.br', 'quran-engine/edition-data-release', 7, QCF_V1_SEMANTICS),
})

function version(value) {
  if (typeof value !== 'string' || !VERSION.test(value)) throw new RangeError('Invalid QVP data version')
  return value.startsWith('v') ? value : `v${value}`
}

function origin(value) {
  if (typeof value !== 'string' || !value) throw new TypeError('Invalid QVP data origin')
  let url
  try { url = new URL(value) } catch { throw new TypeError('Invalid QVP data origin') }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash)
    throw new TypeError('Invalid QVP data origin')
  return url.href.replace(/\/+$/, '')
}

function number(value, maximum, noun) {
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) throw new RangeError(`Invalid ${noun}`)
  return String(value).padStart(3, '0')
}

function format(value) {
  if (value !== 'qvp' && value !== 'svg') throw new RangeError('Invalid Surah-name format')
  return value
}

/**
 * Gets immutable URLs for one exact QVP edition and version.
 *
 * A source is an indivisible asset set: do not mix its pages, sidecars, atlas, or Surah names
 * with another source.
 */
export function qvpSource(key, releaseVersion, { origin: releaseOrigin = CDN_ORIGIN } = {}) {
  const definition = qvpEditions[key]
  if (!definition) throw new RangeError('Unknown QVP data edition')
  const normalizedVersion = version(releaseVersion)
  const normalizedOrigin = origin(releaseOrigin)
  const familyUrl = `${normalizedOrigin}/${definition.family}`
  const baseUrl = `${familyUrl}/${normalizedVersion}`
  const source = {
    [SOURCE]: true,
    ...definition,
    version: normalizedVersion,
    baseUrl,
    latestUrl: `${familyUrl}/latest.json`,
    manifestUrl: `${baseUrl}/manifest.json`,
    versionUrl: `${baseUrl}/VERSION.json`,
    bundleUrl: `${baseUrl}/${definition.bundleName}`,
    atlasUrl: `${baseUrl}/atlas.qva`,
    atlasMetadataUrl: `${baseUrl}/atlas.json`,
    surahNameFontUrl: `${baseUrl}/surah-names/surah-names.woff2`,
    surahNameStylesheetUrl: `${baseUrl}/surah-names/surah-names.css`,
    surahNameMapUrl: `${baseUrl}/surah-names/map.json`,
    pageUrl(page) { return `${baseUrl}/${number(page, definition.pageCount, 'Quran page')}.qvp` },
    wordSidecarUrl(page) { return `${baseUrl}/${number(page, definition.pageCount, 'Quran page')}.words.json` },
    surahNameUrl(surah, assetFormat = 'qvp') {
      const kind = format(assetFormat)
      return `${baseUrl}/surah-names/${kind}/${number(surah, definition.surahCount, 'Surah')}.${kind}`
    },
    surahNamesUrl(assetFormat = 'qvp') {
      const kind = format(assetFormat)
      return `${baseUrl}/surah-names/${kind}/all.${kind}`
    },
  }
  return Object.freeze(source)
}

function object(value, message) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new TypeError(message)
  return value
}

function digest(value, message) {
  if (typeof value !== 'string' || !SHA256.test(value)) throw new Error(message)
}

function sameNumberMap(value, expected) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false
  const names = Object.keys(expected).sort()
  return Object.keys(value).sort().join('\0') === names.join('\0') &&
    names.every(name => value[name] === expected[name])
}

function releaseCount(release, primary, nested) {
  return release[primary] ?? release.qvp?.[nested]
}

/**
 * Verifies that a CDN manifest is the complete asset set named by a QVP source.
 *
 * This validates identity, inventory, sizes, and digest syntax. A downloader must still hash
 * each response and compare it with the matching `files` entry before storing the bytes.
 */
export function verifyQvpManifest(source, manifest) {
  if (!source || source[SOURCE] !== true) throw new TypeError('Invalid QVP data source')
  object(manifest, 'Invalid QVP manifest')
  const release = object(manifest.release, 'Invalid QVP release metadata')
  const bundle = object(manifest.bundle, 'Invalid QVP bundle metadata')
  if (manifest.version !== source.version || manifest.base !== `${source.baseUrl}/` || manifest.encoding !== 'identity')
    throw new Error('QVP manifest identity differs')

  const packageName = release.package ?? release.name
  if (packageName !== source.packageName || release.version !== source.version.slice(1) || release.format_version !== source.formatVersion)
    throw new Error('QVP release identity differs')
  if (source.releaseSchema !== null &&
      (release.schema !== source.releaseSchema || release.schema_version !== source.releaseSchemaVersion))
    throw new Error('QVP release schema differs')
  if (source.semantics !== null) {
    const evidence = object(release.source, 'Invalid QVP source evidence')
    const waqf = object(evidence.waqf, 'Invalid QVP waqf evidence')
    const geometry = object(evidence.base_path_geometry, 'Invalid QVP path-geometry evidence')
    const divisions = object(evidence.division_sajdah, 'Invalid QVP division/sajdah evidence')
    const expected = source.semantics
    if (
      evidence.summary_schema_version !== expected.sourceSchemaVersion ||
      evidence.qualification !== expected.qualification ||
      evidence.hq_words !== expected.hqWords ||
      evidence.fallback_words !== expected.fallbackWords ||
      evidence.pages_with_hq !== expected.pagesWithHq ||
      evidence.summary_sha256 !== expected.summarySha256 ||
      evidence.pages_sha256 !== expected.pagesSha256 ||
      evidence.qualified_geometry_pages_sha256 !== expected.qualifiedGeometryPagesSha256 ||
      evidence.source_candidate_tree_sha256 !== expected.candidateTreeSha256 ||
      evidence.source_manifest_sha256 !== expected.sourceManifestSha256 ||
      evidence.source_records_sha256 !== expected.sourceRecordsSha256 ||
      evidence.optical_calibration_sha256 !== expected.opticalCalibrationSha256 ||
      !sameNumberMap(evidence.source_exclusions, expected.sourceExclusions) ||
      waqf.separate_source_glyphs !== expected.waqfPaths ||
      waqf.fused_source_glyphs !== expected.fusedWaqf ||
      geometry.qualification !== 'base-order-preserved' ||
      geometry.base_paths !== expected.basePaths ||
      geometry.emitted_paths !== expected.emittedPaths ||
      geometry.restored_paths !== expected.restoredPaths ||
      divisions.division_starts !== expected.divisionStarts ||
      divisions.printed_rubu_al_hizb !== expected.printedDivisions ||
      divisions.unprinted_rubu_al_hizb !== expected.unprintedDivisions ||
      divisions.mapped_sajdah_source_glyphs !== expected.mappedSajdah ||
      divisions.restored_sajdah_source_glyphs !== expected.restoredSajdah ||
      divisions.sajdah_marks !== expected.sajdahMarks
    ) throw new Error('QVP release semantic evidence differs')
  }
  if (release.edition !== undefined && release.edition !== source.edition) throw new Error('QVP release edition differs')
  if (release.print_year_hijri !== undefined && release.print_year_hijri !== source.printYearHijri)
    throw new Error('QVP release print year differs')
  if (releaseCount(release, 'pages', 'pages') !== source.pageCount) throw new Error('QVP release page count differs')
  const surahNames = releaseCount(release, 'surah_names', 'surah_names')
  if (surahNames !== undefined && surahNames !== source.surahCount) throw new Error('QVP release Surah-name count differs')

  if (bundle.name !== source.bundleName || !Number.isSafeInteger(bundle.bytes) || bundle.bytes <= 0)
    throw new Error('QVP bundle identity differs')
  digest(bundle.sha256, 'Invalid QVP bundle digest')
  if (bundle.decoded !== undefined) {
    object(bundle.decoded, 'Invalid decoded QVP bundle metadata')
    if (!Number.isSafeInteger(bundle.decoded.bytes) || bundle.decoded.bytes <= 0)
      throw new Error('Invalid decoded QVP bundle size')
    digest(bundle.decoded.sha256, 'Invalid decoded QVP bundle digest')
  }
  if (!Array.isArray(manifest.files)) throw new TypeError('Invalid QVP file inventory')

  const names = new Set()
  const files = new Map()
  for (const file of manifest.files) {
    object(file, 'Invalid QVP file record')
    if (Object.keys(file).sort().join(',') !== 'bytes,name,sha256' || typeof file.name !== 'string' || !file.name || file.name.startsWith('/') || file.name.includes('\\') || file.name.split('/').some(part => !part || part === '.' || part === '..'))
      throw new Error('Invalid QVP file name')
    if (names.has(file.name)) throw new Error(`Duplicate QVP file: ${file.name}`)
    if (
      !Number.isSafeInteger(file.bytes) ||
      file.bytes < 0 ||
      (file.bytes === 0 && file.name !== 'NOTICE.txt')
    ) throw new Error(`Invalid QVP file size: ${file.name}`)
    digest(file.sha256, `Invalid QVP file digest: ${file.name}`)
    names.add(file.name)
    files.set(file.name, file)
  }

  const declared = release.payload_files ?? release.files
  if (declared !== undefined) {
    object(declared, 'Invalid QVP release digest inventory')
    for (const [name, expected] of Object.entries(declared)) {
      digest(expected, `Invalid QVP release digest: ${name}`)
      if (files.get(name)?.sha256 !== expected)
        throw new Error(`QVP release digest differs: ${name}`)
    }
  }
  if (release.qvp !== undefined) {
    object(release.qvp, 'Invalid QVP release inventory')
    const expectedDataFiles = source.pageCount * 2 + source.surahCount * 2 + 7
    if (
      release.qvp.files !== expectedDataFiles ||
      release.qvp.pages !== source.pageCount ||
      release.qvp.word_sidecars !== source.pageCount ||
      release.qvp.logical_words !== source.wordCount ||
      release.qvp.surah_names !== source.surahCount
    ) throw new Error('QVP release inventory differs')
  }

  for (let page = 1; page <= source.pageCount; page++) {
    const name = String(page).padStart(3, '0')
    if (!names.has(`${name}.qvp`) || !names.has(`${name}.words.json`)) throw new Error(`Incomplete QVP page: ${name}`)
  }
  for (let surah = 1; surah <= source.surahCount; surah++) {
    const name = String(surah).padStart(3, '0')
    if (!names.has(`surah-names/qvp/${name}.qvp`) || !names.has(`surah-names/svg/${name}.svg`))
      throw new Error(`Incomplete QVP Surah name: ${name}`)
  }
  const fixed = [
    'atlas.qva',
    'atlas.json',
    'VERSION.json',
    'surah-names/qvp/all.qvp',
    'surah-names/svg/all.svg',
    'surah-names/surah-names.woff2',
    'surah-names/surah-names.css',
    'surah-names/map.json',
  ]
  for (const name of fixed) if (!names.has(name)) throw new Error(`Missing QVP asset: ${name}`)

  // VERSION.json cannot contain its own digest. It is required in the CDN inventory but is
  // deliberately excluded from the release's self-digest map.
  const requiredData = new Set(fixed.filter(name => name !== 'VERSION.json'))
  for (let page = 1; page <= source.pageCount; page++) {
    const name = String(page).padStart(3, '0')
    requiredData.add(`${name}.qvp`)
    requiredData.add(`${name}.words.json`)
  }
  for (let surah = 1; surah <= source.surahCount; surah++) {
    const name = String(surah).padStart(3, '0')
    requiredData.add(`surah-names/qvp/${name}.qvp`)
    requiredData.add(`surah-names/svg/${name}.svg`)
  }
  if (declared !== undefined)
    for (const name of requiredData)
      if (!Object.hasOwn(declared, name)) throw new Error(`Missing QVP release digest: ${name}`)

  const allowed = new Set([...requiredData, 'VERSION.json', 'README.md', 'NOTICE.txt', 'LICENSE.txt'])
  for (const name of names) if (!allowed.has(name)) throw new Error(`Unexpected QVP asset: ${name}`)
  return manifest
}
