/* JNI shim: net.quranpedia.qvp.QvpNative ↔ qvp.h (complete surface).
 * Marshalling rules: structs go to the JVM as FloatArray/IntArray records (documented per function
 * in QvpNative.kt); colours stay uint32 inside IntArray; QvpStr → String; targets/selectors come in
 * as IntArray {kind, a, b, c, words...}. */
#include <jni.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>
#include "qvp.h"

#define FN(name) JNIEXPORT JNICALL Java_net_quranpedia_qvp_QvpNative_##name
#define PG(h) ((QvpPage*)(intptr_t)(h))
#define AT(h) ((QvpAtlas*)(intptr_t)(h))

static jfloatArray floats(JNIEnv* env, const float* v, jsize n) { jfloatArray a = (*env)->NewFloatArray(env, n); if (n) (*env)->SetFloatArrayRegion(env, a, 0, n, v); return a; }
static jintArray ints(JNIEnv* env, const jint* v, jsize n) { jintArray a = (*env)->NewIntArray(env, n); if (n) (*env)->SetIntArrayRegion(env, a, 0, n, v); return a; }
static jstring utf8(JNIEnv* env, const uint8_t* p, uint32_t n) {
    /* NewStringUTF wants modified UTF-8; go through a byte[] + String(bytes, "UTF-8") for correctness */
    jbyteArray b = (*env)->NewByteArray(env, n);
    (*env)->SetByteArrayRegion(env, b, 0, n, (const jbyte*)p);
    jclass sc = (*env)->FindClass(env, "java/lang/String");
    jmethodID ctor = (*env)->GetMethodID(env, sc, "<init>", "([BLjava/lang/String;)V");
    jstring enc = (*env)->NewStringUTF(env, "UTF-8");
    jstring s = (jstring)(*env)->NewObject(env, sc, ctor, b, enc);
    (*env)->DeleteLocalRef(env, b); (*env)->DeleteLocalRef(env, enc); (*env)->DeleteLocalRef(env, sc);
    return s;
}
static jstring qstr(JNIEnv* env, QvpStr s) { return utf8(env, s.ptr, s.len); }
/* jstring → UTF-8 bytes (caller frees) */
static uint8_t* jbytes(JNIEnv* env, jstring s, uint32_t* len) {
    if (!s) { *len = 0; return NULL; }
    jclass sc = (*env)->FindClass(env, "java/lang/String");
    jmethodID gb = (*env)->GetMethodID(env, sc, "getBytes", "(Ljava/lang/String;)[B");
    jstring enc = (*env)->NewStringUTF(env, "UTF-8");
    jbyteArray b = (jbyteArray)(*env)->CallObjectMethod(env, s, gb, enc);
    jsize n = (*env)->GetArrayLength(env, b);
    uint8_t* out = (uint8_t*)malloc(n + 1);
    (*env)->GetByteArrayRegion(env, b, 0, n, (jbyte*)out);
    out[n] = 0; *len = (uint32_t)n;
    (*env)->DeleteLocalRef(env, b); (*env)->DeleteLocalRef(env, enc); (*env)->DeleteLocalRef(env, sc);
    return out;
}
static float fbits(jint v) { float f; memcpy(&f, &v, 4); return f; }
static jint ibits(float f) { jint v; memcpy(&v, &f, 4); return v; }

/* target IntArray: {kind, a, b, c, words...} */
typedef struct { QvpTarget t; jint* buf; jintArray arr; } TargetIn;
static TargetIn target_in(JNIEnv* env, jintArray arr) {
    TargetIn ti; memset(&ti, 0, sizeof ti); ti.arr = arr;
    jsize n = (*env)->GetArrayLength(env, arr);
    ti.buf = (*env)->GetIntArrayElements(env, arr, NULL);
    ti.t.kind = (uint8_t)ti.buf[0]; ti.t.a = ti.buf[1]; ti.t.b = ti.buf[2]; ti.t.c = ti.buf[3];
    ti.t.words = (const uint32_t*)(ti.buf + 4); ti.t.n_words = n > 4 ? n - 4 : 0;
    return ti;
}
static void target_done(JNIEnv* env, TargetIn* ti) { (*env)->ReleaseIntArrayElements(env, ti->arr, ti->buf, JNI_ABORT); }
static QvpSelector sel_in(JNIEnv* env, jintArray arr) {
    jint v[4] = {0, 0, 0, 0}; (*env)->GetIntArrayRegion(env, arr, 0, 4, v);
    QvpSelector s = { (uint8_t)v[0], (uint32_t)v[1], (uint32_t)v[2], (uint32_t)v[3] }; return s;
}
/* style ints {mode, height, ink, band, ms, layer}; floats {padX, padY, radius, seam} */
static QvpHighlightStyle hl_in(JNIEnv* env, jintArray ii, jfloatArray ff) {
    jint v[6]; jfloat f[4]; (*env)->GetIntArrayRegion(env, ii, 0, 6, v); (*env)->GetFloatArrayRegion(env, ff, 0, 4, f);
    QvpHighlightStyle s = { (uint8_t)v[0], (uint8_t)v[1], (uint32_t)v[2], (uint32_t)v[3], f[0], f[1], f[2], f[3], (uint32_t)v[4], v[5] }; return s;
}
static jintArray boxes_out(JNIEnv* env, const QvpBox* b, uint32_t n) {
    jint* v = (jint*)malloc(n * 8 * sizeof(jint));
    for (uint32_t i = 0; i < n; i++) { const QvpBox* x = &b[i]; jint* o = v + i * 8; o[0] = x->id; o[1] = x->line; o[2] = ibits(x->x0); o[3] = ibits(x->y0); o[4] = ibits(x->x1); o[5] = ibits(x->y1); o[6] = (jint)x->color; o[7] = ibits(x->radius); }
    jintArray a = ints(env, v, n * 8); free(v); return a;
}

/* ───────── page ───────── */
jlong FN(pageLoad)(JNIEnv* env, jclass c, jbyteArray bytes) {
    jsize n = (*env)->GetArrayLength(env, bytes); jbyte* p = (*env)->GetByteArrayElements(env, bytes, NULL);
    QvpPage* page = qvp_page_load((const uint8_t*)p, (size_t)n);
    (*env)->ReleaseByteArrayElements(env, bytes, p, JNI_ABORT);
    return (jlong)(intptr_t)page;
}
void FN(pageFree)(JNIEnv* env, jclass c, jlong h) { qvp_page_free(PG(h)); }
jfloatArray FN(pageInfo)(JNIEnv* env, jclass c, jlong h) {
    QvpPageInfo i; qvp_page_info(PG(h), &i);
    float v[8] = { i.width, i.height, (float)i.page, (float)i.n_lines, (float)i.n_ayahs, (float)i.n_words, (float)i.n_paths, (float)i.n_decos };
    return floats(env, v, 8);
}
jbyteArray FN(geomOps)(JNIEnv* env, jclass c, jlong h) { QvpGeometry g; qvp_geometry(PG(h), &g); jbyteArray a = (*env)->NewByteArray(env, g.ops_len); (*env)->SetByteArrayRegion(env, a, 0, g.ops_len, (const jbyte*)g.ops); return a; }
jfloatArray FN(geomPts)(JNIEnv* env, jclass c, jlong h) { QvpGeometry g; qvp_geometry(PG(h), &g); return floats(env, g.pts, g.pts_len); }
jintArray FN(geomTable)(JNIEnv* env, jclass c, jlong h) { QvpGeometry g; qvp_geometry(PG(h), &g); return ints(env, (const jint*)g.table, g.n_paths * 8); }
/* {sura, ayah, word, lineNo, ayahIdx, lineIdx, x0, y0, x1, y1, firstPath, nPaths} */
jfloatArray FN(wordInfo)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpWordInfo w; if (!qvp_word_info(PG(h), i, &w)) return NULL;
    float v[12] = { w.sura, w.ayah, w.word, w.line_no, (float)w.ayah_idx, (float)w.line_idx, w.x0, w.y0, w.x1, w.y1, (float)w.first_path, (float)w.n_paths };
    return floats(env, v, 12);
}
jstring FN(wordText)(JNIEnv* env, jclass c, jlong h, jint i) { QvpWordInfo w; if (!qvp_word_info(PG(h), i, &w)) return NULL; return qstr(env, w.text); }
jstring FN(wordForm)(JNIEnv* env, jclass c, jlong h, jint i, jint form) { QvpStr s; if (!qvp_word_form(PG(h), i, (uint8_t)form, &s)) return NULL; return qstr(env, s); }
/* {sura, ayah, part, parts, flags, rub, firstWord, nWords, markerDeco(-1), x0, y0, x1, y1} */
jfloatArray FN(ayahInfo)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpAyahInfo a; if (!qvp_ayah_info(PG(h), i, &a)) return NULL;
    float v[13] = { a.sura, a.ayah, a.part, a.parts, a.flags, a.rub, (float)a.first_word, (float)a.n_words, a.marker_deco == QVP_NONE ? -1.f : (float)a.marker_deco, a.x0, a.y0, a.x1, a.y1 };
    return floats(env, v, 13);
}
/* {lineNo, isHeader, firstWord, nWords, x0, y0, x1, y1, bandY0, bandY1, centre} */
jfloatArray FN(lineInfo)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpLineInfo l; if (!qvp_line_info(PG(h), i, &l)) return NULL;
    float v[11] = { l.line_no, l.is_header, (float)l.first_word, (float)l.n_words, l.x0, l.y0, l.x1, l.y1, l.band_y0, l.band_y1, l.centre };
    return floats(env, v, 11);
}
/* {kind, sura, ayah, line, x0, y0, x1, y1, firstPath, nPaths} */
jfloatArray FN(decoInfo)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpDecoInfo d; if (!qvp_deco_info(PG(h), i, &d)) return NULL;
    float v[10] = { d.kind, d.sura, d.ayah, (float)d.line, d.x0, d.y0, d.x1, d.y1, (float)d.first_path, (float)d.n_paths };
    return floats(env, v, 10);
}
jstring FN(decoText)(JNIEnv* env, jclass c, jlong h, jint i) { QvpDecoInfo d; if (!qvp_deco_info(PG(h), i, &d)) return NULL; return qstr(env, d.text); }
jint FN(findWord)(JNIEnv* env, jclass c, jlong h, jint s, jint a, jint w) { return qvp_find_word(PG(h), (uint16_t)s, (uint16_t)a, (uint16_t)w); }
jintArray FN(resolve)(JNIEnv* env, jclass c, jlong h, jintArray t) {
    TargetIn ti = target_in(env, t);
    uint32_t n = qvp_resolve(PG(h), &ti.t, NULL, 0);
    uint32_t* buf = (uint32_t*)malloc((n ? n : 1) * 4); qvp_resolve(PG(h), &ti.t, buf, n);
    target_done(env, &ti); jintArray a = ints(env, (const jint*)buf, n); free(buf); return a;
}
jfloat FN(naturalPitch)(JNIEnv* env, jclass c, jlong h) { return qvp_natural_pitch(PG(h)); }

/* ───────── metadata ───────── */
jint FN(surahsCount)(JNIEnv* env, jclass c, jlong h) { return (jint)qvp_surahs_count(PG(h)); }
/* {number, ayahCount, hasBanner, hasBasmalah, place, bannerDeco(-1)} */
jfloatArray FN(surahNums)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpSurahInfo s; if (!qvp_surah_at(PG(h), i, &s)) return NULL;
    float v[6] = { s.number, s.ayah_count, s.has_banner, s.has_basmalah, s.place, s.banner_deco == QVP_NONE ? -1.f : (float)s.banner_deco };
    return floats(env, v, 6);
}
jobjectArray FN(surahNames)(JNIEnv* env, jclass c, jlong h, jint i) {
    QvpSurahInfo s; if (!qvp_surah_at(PG(h), i, &s)) return NULL;
    jobjectArray arr = (*env)->NewObjectArray(env, 3, (*env)->FindClass(env, "java/lang/String"), NULL);
    (*env)->SetObjectArrayElement(env, arr, 0, qstr(env, s.arabic)); (*env)->SetObjectArrayElement(env, arr, 1, qstr(env, s.latin)); (*env)->SetObjectArrayElement(env, arr, 2, qstr(env, s.english));
    return arr;
}
/* 6 per: kind, line, n, sura, ayah, ayahIdx */
jintArray FN(divisions)(JNIEnv* env, jclass c, jlong h) {
    QvpDivision d[64]; uint32_t n = qvp_divisions(PG(h), d, 64); if (n > 64) n = 64;
    jint v[64 * 6]; for (uint32_t i = 0; i < n; i++) { v[i*6] = d[i].kind; v[i*6+1] = d[i].line; v[i*6+2] = d[i].n; v[i*6+3] = d[i].sura; v[i*6+4] = d[i].ayah; v[i*6+5] = (jint)d[i].ayah_idx; }
    return ints(env, v, n * 6);
}
/* 9 per: deco, sura, ayah, line, cx, cy, r, ornamentPath(-1), numeralPath(-1) */
jfloatArray FN(markers)(JNIEnv* env, jclass c, jlong h) {
    QvpMarker m[128]; uint32_t n = qvp_markers(PG(h), m, 128); if (n > 128) n = 128;
    float* v = (float*)malloc(n * 9 * sizeof(float) + 4);
    for (uint32_t i = 0; i < n; i++) { float* o = v + i * 9; o[0] = (float)m[i].deco; o[1] = m[i].sura; o[2] = m[i].ayah; o[3] = (float)m[i].line; o[4] = m[i].cx; o[5] = m[i].cy; o[6] = m[i].r; o[7] = m[i].ornament_path == QVP_NONE ? -1.f : (float)m[i].ornament_path; o[8] = m[i].numeral_path == QVP_NONE ? -1.f : (float)m[i].numeral_path; }
    jfloatArray a = floats(env, v, n * 9); free(v); return a;
}
/* 8 per: deco, sura, ayah, juz, hizb, nisf, rub, rubInHizb */
jintArray FN(rosettes)(JNIEnv* env, jclass c, jlong h) {
    QvpRosette r[32]; uint32_t n = qvp_rosettes(PG(h), r, 32); if (n > 32) n = 32;
    jint v[32 * 8]; for (uint32_t i = 0; i < n; i++) { jint* o = v + i * 8; o[0] = (jint)r[i].deco; o[1] = r[i].sura; o[2] = r[i].ayah; o[3] = r[i].juz; o[4] = r[i].hizb; o[5] = r[i].nisf; o[6] = r[i].rub; o[7] = r[i].rub_in_hizb; }
    return ints(env, v, n * 8);
}
/* 4 per: deco, sura, ayah, signPath */
jintArray FN(sajdahs)(JNIEnv* env, jclass c, jlong h) {
    QvpSajdah s[16]; uint32_t n = qvp_sajdahs(PG(h), s, 16); if (n > 16) n = 16;
    jint v[16 * 4]; for (uint32_t i = 0; i < n; i++) { v[i*4] = (jint)s[i].deco; v[i*4+1] = s[i].sura; v[i*4+2] = s[i].ayah; v[i*4+3] = (jint)s[i].sign_path; }
    return ints(env, v, n * 4);
}
jintArray FN(ayahKeys)(JNIEnv* env, jclass c, jlong h) { uint32_t k[512]; uint32_t n = qvp_ayah_keys(PG(h), k, 512); if (n > 512) n = 512; return ints(env, (const jint*)k, n); }
jintArray FN(ayahWordCount)(JNIEnv* env, jclass c, jlong h, jint s, jint a) { uint32_t complete = 0; uint32_t n = qvp_ayah_word_count(PG(h), (uint16_t)s, (uint16_t)a, &complete); jint v[2] = { (jint)n, (jint)complete }; return ints(env, v, 2); }
jintArray FN(reciteMap)(JNIEnv* env, jclass c, jlong h, jint s, jint a, jint nseg) {
    uint32_t buf[4096]; int32_t n = qvp_recite_map(PG(h), (uint16_t)s, (uint16_t)a, (uint32_t)nseg, buf, 4096);
    if (n < 0) return NULL; if (n > 4096) n = 4096; return ints(env, (const jint*)buf, n);
}
jstring FN(wordLabel)(JNIEnv* env, jclass c, jlong h, jint i) { QvpStr s; qvp_word_label(PG(h), i, &s); return qstr(env, s); }
jstring FN(ayahLabel)(JNIEnv* env, jclass c, jlong h, jint i) { QvpStr s; qvp_ayah_label(PG(h), i, &s); return qstr(env, s); }

/* ───────── text & search ───────── */
jstring FN(textTarget)(JNIEnv* env, jclass c, jlong h, jintArray t, jint form, jstring wsep, jstring lsep) {
    TargetIn ti = target_in(env, t); uint32_t wn, ln; uint8_t* w = jbytes(env, wsep, &wn); uint8_t* l = jbytes(env, lsep, &ln);
    QvpStr s; qvp_text_target(PG(h), &ti.t, (uint8_t)form, w, wn, l, ln, &s);
    jstring r = qstr(env, s); free(w); free(l); target_done(env, &ti); return r;
}
/* 3 per: word, index, loose */
jintArray FN(search)(JNIEnv* env, jclass c, jlong h, jstring q, jint form, jint mode, jboolean normalize, jboolean loose, jint limit) {
    uint32_t qn; uint8_t* qb = jbytes(env, q, &qn);
    QvpMatch m[1024]; uint32_t n = qvp_search(PG(h), qb, qn, (uint8_t)form, (uint8_t)mode, normalize ? 1 : 0, loose ? 1 : 0, (uint32_t)limit, m, 1024); free(qb);
    if (n > 1024) n = 1024;
    jint* v = (jint*)malloc((n ? n : 1) * 3 * sizeof(jint)); for (uint32_t i = 0; i < n; i++) { v[i*3] = (jint)m[i].word; v[i*3+1] = (jint)m[i].index; v[i*3+2] = (jint)m[i].loose; }
    jintArray a = ints(env, v, n * 3); free(v); return a;
}
jstring FN(arabic)(JNIEnv* env, jclass c, jint kind, jstring in) { uint32_t n; uint8_t* b = jbytes(env, in, &n); QvpStr s; qvp_arabic((uint8_t)kind, b, n, &s); jstring r = qstr(env, s); free(b); return r; }
jstring FN(citation)(JNIEnv* env, jclass c, jlong h, jintArray words) {
    jsize n = (*env)->GetArrayLength(env, words); jint* w = (*env)->GetIntArrayElements(env, words, NULL);
    QvpStr s; qvp_citation(PG(h), (const uint32_t*)w, (uint32_t)n, &s); (*env)->ReleaseIntArrayElements(env, words, w, JNI_ABORT); return qstr(env, s);
}
jint FN(attachWords)(JNIEnv* env, jclass c, jlong h, jbyteArray json) {
    jsize n = (*env)->GetArrayLength(env, json); jbyte* p = (*env)->GetByteArrayElements(env, json, NULL);
    jint r = qvp_attach_words(PG(h), (const uint8_t*)p, (uint32_t)n); (*env)->ReleaseByteArrayElements(env, json, p, JNI_ABORT); return r;
}
jboolean FN(hasForm)(JNIEnv* env, jclass c, jlong h, jint form) { return qvp_has_form(PG(h), (uint8_t)form) ? JNI_TRUE : JNI_FALSE; }

/* ───────── hit testing ───────── */
static jintArray hit_out(JNIEnv* env, int ok, QvpHit* hit) { if (!ok) return NULL; jint v[3] = { hit->word == QVP_NONE ? -1 : (jint)hit->word, hit->path == QVP_NONE ? -1 : (jint)hit->path, hit->deco == QVP_NONE ? -1 : (jint)hit->deco }; return ints(env, v, 3); }
jintArray FN(hitTest)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y) { QvpHit hit; return hit_out(env, qvp_hit_test(PG(h), x, y, &hit), &hit); }
jintArray FN(hitTestView)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y) { QvpHit hit; return hit_out(env, qvp_hit_test_view(PG(h), x, y, &hit), &hit); }
/* {word(-1), path(-1), deco(-1), line, distance, exact} */
static jfloatArray hitex_out(JNIEnv* env, int ok, QvpHitEx* h) {
    if (!ok) return NULL; float v[6] = { h->word == QVP_NONE ? -1.f : (float)h->word, h->path == QVP_NONE ? -1.f : (float)h->path, h->deco == QVP_NONE ? -1.f : (float)h->deco, (float)h->line, h->distance, (float)h->exact }; return floats(env, v, 6);
}
jfloatArray FN(hitTestEx)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y, jfloat maxDist, jfloat gapBias, jboolean exactFirst) { QvpHitOptions o = { maxDist, gapBias, exactFirst ? 1u : 0u }; QvpHitEx r; return hitex_out(env, qvp_hit_test_ex(PG(h), x, y, &o, &r), &r); }
jfloatArray FN(hitTestViewEx)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y, jfloat maxDist, jfloat gapBias, jboolean exactFirst) { QvpHitOptions o = { maxDist, gapBias, exactFirst ? 1u : 0u }; QvpHitEx r; return hitex_out(env, qvp_hit_test_view_ex(PG(h), x, y, &o, &r), &r); }
/* 7 per: line, lineNo, y0, y1, mid, inkY0, inkY1 */
jfloatArray FN(lineBands)(JNIEnv* env, jclass c, jlong h) {
    QvpLineBand b[64]; uint32_t n = qvp_line_bands(PG(h), b, 64); if (n > 64) n = 64;
    float v[64 * 7]; for (uint32_t i = 0; i < n; i++) { float* o = v + i * 7; o[0] = (float)b[i].line; o[1] = (float)b[i].line_no; o[2] = b[i].y0; o[3] = b[i].y1; o[4] = b[i].mid; o[5] = b[i].ink_y0; o[6] = b[i].ink_y1; }
    return floats(env, v, n * 7);
}
/* 10 per: word, line, x0, y0, x1, y1, inkX0, inkY0, inkX1, inkY1 */
jfloatArray FN(hitBoxes)(JNIEnv* env, jclass c, jlong h, jfloat gapBias) {
    uint32_t n = qvp_hit_boxes(PG(h), gapBias, NULL, 0); QvpHitBox* b = (QvpHitBox*)malloc((n ? n : 1) * sizeof(QvpHitBox)); qvp_hit_boxes(PG(h), gapBias, b, n);
    float* v = (float*)malloc((n ? n : 1) * 10 * sizeof(float));
    for (uint32_t i = 0; i < n; i++) { float* o = v + i * 10; o[0] = (float)b[i].word; o[1] = (float)b[i].line; o[2] = b[i].x0; o[3] = b[i].y0; o[4] = b[i].x1; o[5] = b[i].y1; o[6] = b[i].ink_x0; o[7] = b[i].ink_y0; o[8] = b[i].ink_x1; o[9] = b[i].ink_y1; }
    jfloatArray a = floats(env, v, n * 10); free(v); free(b); return a;
}

/* ───────── layout ───────── */
/* spec {vw, vh, padTop, padBottom, padLeft, padRight, lineSpacing, lineGap, fillHeight, nominal}
   → {scale, ox, oy, contentW, contentH, pitch, nLines, then nLines × (dy, slotTop, slotBottom)} */
jfloatArray FN(layout)(JNIEnv* env, jclass c, jlong h, jfloatArray spec) {
    jfloat f[10]; (*env)->GetFloatArrayRegion(env, spec, 0, 10, f);
    QvpLayoutSpec s = { f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8] > 0.5f ? 1u : 0u, (uint32_t)f[9] };
    QvpLayout l; qvp_layout(PG(h), &s, &l);
    jsize n = 7 + l.n_lines * 3; float* v = (float*)malloc(n * sizeof(float));
    v[0] = l.scale; v[1] = l.ox; v[2] = l.oy; v[3] = l.content_w; v[4] = l.content_h; v[5] = l.pitch; v[6] = (float)l.n_lines;
    memcpy(v + 7, l.lines, l.n_lines * 3 * sizeof(float));
    jfloatArray a = floats(env, v, n); free(v); return a;
}
jfloat FN(gapToFill)(JNIEnv* env, jclass c, jfloat pw, jfloat ph, jint lines, jfloat vw, jfloat vh, jfloat max) { return qvp_gap_to_fill(pw, ph, (uint32_t)lines, vw, vh, max); }
jfloat FN(wastedFraction)(JNIEnv* env, jclass c, jfloat pw, jfloat ph, jfloat vw, jfloat vh) { return qvp_wasted_fraction(pw, ph, vw, vh); }
jfloatArray FN(wordBoxView)(JNIEnv* env, jclass c, jlong h, jint i) { float v[4]; if (!qvp_word_box_view(PG(h), i, v)) return NULL; return floats(env, v, 4); }

/* ───────── styles ───────── */
jint FN(styleAdd)(JNIEnv* env, jclass c, jlong h, jint layer, jintArray sel, jint rgba, jint ms) { QvpSelector s = sel_in(env, sel); return (jint)qvp_style_add(PG(h), layer, &s, (uint32_t)rgba, (uint32_t)ms); }
jint FN(styleAddTarget)(JNIEnv* env, jclass c, jlong h, jint layer, jintArray t, jint rgba, jint ms) { TargetIn ti = target_in(env, t); jint r = (jint)qvp_style_add_target(PG(h), layer, &ti.t, (uint32_t)rgba, (uint32_t)ms); target_done(env, &ti); return r; }
jint FN(styleRemove)(JNIEnv* env, jclass c, jlong h, jint handle) { return (jint)qvp_style_remove(PG(h), (uint32_t)handle); }
jint FN(styleRepaint)(JNIEnv* env, jclass c, jlong h, jint handle, jint rgba, jint ms) { return (jint)qvp_style_repaint(PG(h), (uint32_t)handle, (uint32_t)rgba, (uint32_t)ms); }
void FN(styleClear)(JNIEnv* env, jclass c, jlong h) { qvp_style_clear(PG(h)); }
void FN(styleClearLayer)(JNIEnv* env, jclass c, jlong h, jint layer) { qvp_style_clear_layer(PG(h), layer); }
void FN(styleDefault)(JNIEnv* env, jclass c, jlong h, jint rgba) { qvp_style_default(PG(h), (uint32_t)rgba); }
jint FN(hide)(JNIEnv* env, jclass c, jlong h, jintArray sel) { QvpSelector s = sel_in(env, sel); return (jint)qvp_hide(PG(h), &s); }
/* theme IntArray: {ink, diacritics, dots, waqf, sifr, marker, numeral, headers, ms, (mark, colour)*} */
jint FN(theme)(JNIEnv* env, jclass c, jlong h, jintArray arr) {
    jsize n = (*env)->GetArrayLength(env, arr); jint* v = (*env)->GetIntArrayElements(env, arr, NULL);
    QvpTheme t = { (uint32_t)v[0], (uint32_t)v[1], (uint32_t)v[2], (uint32_t)v[3], (uint32_t)v[4], (uint32_t)v[5], (uint32_t)v[6], (uint32_t)v[7], (uint32_t)v[8], (const uint32_t*)(v + 9), (uint32_t)((n - 9) / 2) };
    jint r = (jint)qvp_theme(PG(h), &t); (*env)->ReleaseIntArrayElements(env, arr, v, JNI_ABORT); return r;
}
jintArray FN(styleHandles)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_style_handles(PG(h), NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 4); qvp_style_handles(PG(h), b, n); jintArray a = ints(env, (const jint*)b, n); free(b); return a; }

/* ───────── clock & display list ───────── */
jboolean FN(tick)(JNIEnv* env, jclass c, jlong h, jdouble now) { return qvp_tick(PG(h), now) ? JNI_TRUE : JNI_FALSE; }
jintArray FN(paint)(JNIEnv* env, jclass c, jlong h) { QvpPageInfo i; qvp_page_info(PG(h), &i); const uint32_t* col = qvp_paint(PG(h)); return ints(env, (const jint*)col, i.n_paths); }
jintArray FN(styled)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_styled(PG(h), NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 8); qvp_styled(PG(h), b, n); jintArray a = ints(env, (const jint*)b, n * 2); free(b); return a; }
jint FN(colorOf)(JNIEnv* env, jclass c, jlong h, jint p) { return (jint)qvp_color_of(PG(h), (uint32_t)p); }

/* ───────── highlights ───────── */
jint FN(highlight)(JNIEnv* env, jclass c, jlong h, jintArray t, jintArray si, jfloatArray sf) { TargetIn ti = target_in(env, t); QvpHighlightStyle s = hl_in(env, si, sf); jint r = (jint)qvp_highlight(PG(h), &ti.t, &s); target_done(env, &ti); return r; }
jboolean FN(rehighlight)(JNIEnv* env, jclass c, jlong h, jint handle, jintArray t) { TargetIn ti = target_in(env, t); uint32_t r = qvp_rehighlight(PG(h), (uint32_t)handle, &ti.t); target_done(env, &ti); return r ? JNI_TRUE : JNI_FALSE; }
jboolean FN(restyleHighlight)(JNIEnv* env, jclass c, jlong h, jint handle, jintArray si, jfloatArray sf) { QvpHighlightStyle s = hl_in(env, si, sf); return qvp_restyle_highlight(PG(h), (uint32_t)handle, &s) ? JNI_TRUE : JNI_FALSE; }
jboolean FN(unhighlight)(JNIEnv* env, jclass c, jlong h, jint handle) { return qvp_unhighlight(PG(h), (uint32_t)handle) ? JNI_TRUE : JNI_FALSE; }
void FN(clearHighlights)(JNIEnv* env, jclass c, jlong h) { qvp_clear_highlights(PG(h)); }
jintArray FN(highlightHandles)(JNIEnv* env, jclass c, jlong h) { uint32_t b[1024]; uint32_t n = qvp_highlight_handles(PG(h), b, 1024); if (n > 1024) n = 1024; return ints(env, (const jint*)b, n); }
jintArray FN(highlightWords)(JNIEnv* env, jclass c, jlong h, jint handle) { uint32_t n = qvp_highlight_words(PG(h), (uint32_t)handle, NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 4); qvp_highlight_words(PG(h), (uint32_t)handle, b, n); jintArray a = ints(env, (const jint*)b, n); free(b); return a; }
/* boxes: 8 ints per box: id, line, x0, y0, x1, y1 (float bits), color, radius (float bits) */
jintArray FN(highlightBoxes)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_highlight_boxes(PG(h), NULL, 0); QvpBox* b = (QvpBox*)malloc((n ? n : 1) * sizeof(QvpBox)); qvp_highlight_boxes(PG(h), b, n); jintArray a = boxes_out(env, b, n); free(b); return a; }
jintArray FN(bandBoxes)(JNIEnv* env, jclass c, jlong h, jintArray words, jint height, jfloat padX, jfloat padY) {
    jsize n = (*env)->GetArrayLength(env, words); jint* w = (*env)->GetIntArrayElements(env, words, NULL);
    QvpBox b[64]; uint32_t k = qvp_band_boxes(PG(h), (const uint32_t*)w, (uint32_t)n, (uint8_t)height, padX, padY, b, 64); if (k > 64) k = 64;
    (*env)->ReleaseIntArrayElements(env, words, w, JNI_ABORT); return boxes_out(env, b, k);
}

/* ───────── selection ───────── */
void FN(select)(JNIEnv* env, jclass c, jlong h, jint anchor, jint focus) { qvp_select(PG(h), anchor < 0 ? QVP_NONE : (uint32_t)anchor, focus < 0 ? QVP_NONE : (uint32_t)focus); }
jintArray FN(selection)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_selection(PG(h), NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 4); qvp_selection(PG(h), b, n); jintArray a = ints(env, (const jint*)b, n); free(b); return a; }
jstring FN(selectionText)(JNIEnv* env, jclass c, jlong h, jint form, jboolean cite) { QvpStr s; qvp_selection_text(PG(h), (uint8_t)form, cite ? 1 : 0, &s); return qstr(env, s); }

/* ───────── memorisation ───────── */
void FN(mask)(JNIEnv* env, jclass c, jlong h, jintArray t, jint mode) { TargetIn ti = target_in(env, t); qvp_mask(PG(h), &ti.t, (uint8_t)mode); target_done(env, &ti); }
void FN(maskFrom)(JNIEnv* env, jclass c, jlong h, jint wi, jint mode) { qvp_mask_from(PG(h), (uint32_t)wi, (uint8_t)mode); }
void FN(maskOptions)(JNIEnv* env, jclass c, jlong h, jint color, jfloat px, jfloat py, jfloat radius, jboolean reverse) { qvp_mask_options(PG(h), (uint32_t)color, px, py, radius, reverse ? 1 : 0); }
jint FN(revealNext)(JNIEnv* env, jclass c, jlong h, jint n) { return (jint)qvp_reveal_next(PG(h), (uint32_t)n); }
jint FN(hideBack)(JNIEnv* env, jclass c, jlong h, jint n) { return (jint)qvp_hide_back(PG(h), (uint32_t)n); }
jboolean FN(revealWord)(JNIEnv* env, jclass c, jlong h, jint wi) { return qvp_reveal_word(PG(h), (uint32_t)wi) ? JNI_TRUE : JNI_FALSE; }
jboolean FN(hideWord)(JNIEnv* env, jclass c, jlong h, jint wi) { return qvp_hide_word(PG(h), (uint32_t)wi) ? JNI_TRUE : JNI_FALSE; }
void FN(revealAll)(JNIEnv* env, jclass c, jlong h) { qvp_reveal_all(PG(h)); }
void FN(hideAll)(JNIEnv* env, jclass c, jlong h) { qvp_hide_all(PG(h)); }
void FN(unmask)(JNIEnv* env, jclass c, jlong h) { qvp_unmask(PG(h)); }
jintArray FN(maskHidden)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_mask_hidden(PG(h), NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 4); qvp_mask_hidden(PG(h), b, n); jintArray a = ints(env, (const jint*)b, n); free(b); return a; }
jintArray FN(maskWords)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_mask_words(PG(h), NULL, 0); uint32_t* b = (uint32_t*)malloc((n ? n : 1) * 4); qvp_mask_words(PG(h), b, n); jintArray a = ints(env, (const jint*)b, n); free(b); return a; }
jintArray FN(maskBoxes)(JNIEnv* env, jclass c, jlong h) { uint32_t n = qvp_mask_boxes(PG(h), NULL, 0); QvpBox* b = (QvpBox*)malloc((n ? n : 1) * sizeof(QvpBox)); qvp_mask_boxes(PG(h), b, n); jintArray a = boxes_out(env, b, n); free(b); return a; }
jint FN(revealStart)(JNIEnv* env, jclass c, jlong h, jint lit, jboolean byAyah, jint grey, jint ink, jboolean markers, jint ms) { return (jint)qvp_reveal_start(PG(h), (uint32_t)lit, byAyah ? 1 : 0, (uint32_t)grey, (uint32_t)ink, markers ? 1 : 0, (uint32_t)ms); }
jboolean FN(revealGoto)(JNIEnv* env, jclass c, jlong h, jlong at) { return qvp_reveal_goto(PG(h), (int64_t)at) ? JNI_TRUE : JNI_FALSE; }
jlong FN(revealAt)(JNIEnv* env, jclass c, jlong h) { return (jlong)qvp_reveal_at(PG(h)); }
jint FN(revealSteps)(JNIEnv* env, jclass c, jlong h) { return (jint)qvp_reveal_steps(PG(h)); }
jlong FN(revealStepOf)(JNIEnv* env, jclass c, jlong h, jint wi) { return (jlong)qvp_reveal_step_of(PG(h), (uint32_t)wi); }
void FN(revealStop)(JNIEnv* env, jclass c, jlong h) { qvp_reveal_stop(PG(h)); }

/* ───────── crop ───────── */
/* {x0, y0, x1, y1, nWords, markerDeco(-1)} */
jfloatArray FN(cropBox)(JNIEnv* env, jclass c, jlong h, jintArray t, jfloat pad, jboolean keep) {
    TargetIn ti = target_in(env, t); QvpCropBox cb; int ok = qvp_crop_box(PG(h), &ti.t, pad, keep ? 1 : 0, &cb); target_done(env, &ti);
    if (!ok) return NULL; float v[6] = { cb.x0, cb.y0, cb.x1, cb.y1, (float)cb.n_words, cb.marker_deco == QVP_NONE ? -1.f : (float)cb.marker_deco }; return floats(env, v, 6);
}
jstring FN(cropSvg)(JNIEnv* env, jclass c, jlong h, jintArray t, jfloat pad, jboolean keep, jint bg) {
    TargetIn ti = target_in(env, t); QvpStr s; int ok = qvp_crop_svg(PG(h), &ti.t, pad, keep ? 1 : 0, (uint32_t)bg, &s); target_done(env, &ti);
    return ok ? qstr(env, s) : NULL;
}

/* ───────── atlas ───────── */
jlong FN(atlasLoad)(JNIEnv* env, jclass c, jbyteArray bytes) { jsize n = (*env)->GetArrayLength(env, bytes); jbyte* p = (*env)->GetByteArrayElements(env, bytes, NULL); QvpAtlas* a = qvp_atlas_load((const uint8_t*)p, (size_t)n); (*env)->ReleaseByteArrayElements(env, bytes, p, JNI_ABORT); return (jlong)(intptr_t)a; }
void FN(atlasFree)(JNIEnv* env, jclass c, jlong h) { qvp_atlas_free(AT(h)); }
jint FN(atlasPageOf)(JNIEnv* env, jclass c, jlong h, jint s, jint a) { return qvp_atlas_page_of(AT(h), (uint16_t)s, (uint16_t)a); }
jintArray FN(atlasPageRange)(JNIEnv* env, jclass c, jlong h, jint page) { uint16_t o[4]; if (!qvp_atlas_page_range(AT(h), (uint16_t)page, o)) return NULL; jint v[4] = { o[0], o[1], o[2], o[3] }; return ints(env, v, 4); }
jint FN(atlasPages)(JNIEnv* env, jclass c, jlong h) { return (jint)qvp_atlas_pages(AT(h)); }
jint FN(atlasSurahs)(JNIEnv* env, jclass c, jlong h) { return (jint)qvp_atlas_surahs(AT(h)); }
static jobjectArray atlas_surah_out(JNIEnv* env, QvpAtlasSurah* s) {
    /* Strings: n, firstPage, ayahCount, place, arabic, latin, english */
    jobjectArray arr = (*env)->NewObjectArray(env, 7, (*env)->FindClass(env, "java/lang/String"), NULL);
    char buf[16];
    snprintf(buf, sizeof buf, "%u", s->n); (*env)->SetObjectArrayElement(env, arr, 0, (*env)->NewStringUTF(env, buf));
    snprintf(buf, sizeof buf, "%u", s->first_page); (*env)->SetObjectArrayElement(env, arr, 1, (*env)->NewStringUTF(env, buf));
    snprintf(buf, sizeof buf, "%u", s->ayah_count); (*env)->SetObjectArrayElement(env, arr, 2, (*env)->NewStringUTF(env, buf));
    snprintf(buf, sizeof buf, "%u", s->place); (*env)->SetObjectArrayElement(env, arr, 3, (*env)->NewStringUTF(env, buf));
    (*env)->SetObjectArrayElement(env, arr, 4, qstr(env, s->arabic)); (*env)->SetObjectArrayElement(env, arr, 5, qstr(env, s->latin)); (*env)->SetObjectArrayElement(env, arr, 6, qstr(env, s->english));
    return arr;
}
jobjectArray FN(atlasSurah)(JNIEnv* env, jclass c, jlong h, jint n) { QvpAtlasSurah s; if (!qvp_atlas_surah(AT(h), (uint16_t)n, &s)) return NULL; return atlas_surah_out(env, &s); }
jobjectArray FN(atlasSurahAt)(JNIEnv* env, jclass c, jlong h, jint i) { QvpAtlasSurah s; if (!qvp_atlas_surah_at(AT(h), (uint32_t)i, &s)) return NULL; return atlas_surah_out(env, &s); }
jintArray FN(atlasDivision)(JNIEnv* env, jclass c, jlong h, jint kind, jint n) { QvpAtlasRub r; if (!qvp_atlas_division(AT(h), (uint8_t)kind, (uint16_t)n, &r)) return NULL; jint v[4] = { r.rub, r.sura, r.ayah, r.page }; return ints(env, v, 4); }
jint FN(atlasDivisionAt)(JNIEnv* env, jclass c, jlong h, jint kind, jint s, jint a) { return qvp_atlas_division_at(AT(h), (uint8_t)kind, (uint16_t)s, (uint16_t)a); }
jintArray FN(atlasPagesOfJuz)(JNIEnv* env, jclass c, jlong h, jint n) { uint16_t o[2]; if (!qvp_atlas_pages_of_juz(AT(h), (uint16_t)n, o)) return NULL; jint v[2] = { o[0], o[1] }; return ints(env, v, 2); }
jintArray FN(atlasFindSurah)(JNIEnv* env, jclass c, jlong h, jstring text) { uint32_t n; uint8_t* b = jbytes(env, text, &n); uint16_t o[128]; uint32_t k = qvp_atlas_find_surah(AT(h), b, n, o, 128); free(b); if (k > 128) k = 128; jint v[128]; for (uint32_t i = 0; i < k; i++) v[i] = o[i]; return ints(env, v, k); }

/* ───────── names ───────── */
jstring FN(markName)(JNIEnv* env, jclass c, jint m) { QvpStr s; qvp_mark_name((uint8_t)m, &s); return qstr(env, s); }
jstring FN(familyName)(JNIEnv* env, jclass c, jint f) { QvpStr s; qvp_family_name((uint8_t)f, &s); return qstr(env, s); }
jstring FN(kindName)(JNIEnv* env, jclass c, jint k) { QvpStr s; qvp_kind_name((uint8_t)k, &s); return qstr(env, s); }
jstring FN(categoryName)(JNIEnv* env, jclass c, jint k) { QvpStr s; qvp_category_name((uint8_t)k, &s); return qstr(env, s); }
jint FN(markFromName)(JNIEnv* env, jclass c, jstring name) { uint32_t n; uint8_t* b = jbytes(env, name, &n); jint r = qvp_mark_from_name(b, n); free(b); return r; }
jint FN(markCategory)(JNIEnv* env, jclass c, jint m) { return qvp_mark_category((uint8_t)m); }
jint FN(version)(JNIEnv* env, jclass c) { return (jint)qvp_version(); }
