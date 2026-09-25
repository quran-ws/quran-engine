// Canonical Hafs surah names and ayah counts, in printed order.
import rows from './surahs.json' with { type: 'json' }

/** Gets the 114 surahs; array positions are zero-based, surah numbers are one-based. */
export const surahs = Object.freeze(rows.map(Object.freeze))
