/* JNI shim: net.quranpedia.qvp.QvpNative ↔ qvp.h. Geometry is copied into JVM arrays once per page;
   everything else is a small call. Colours are 0xRRGGBBAA as in the C ABI. */
#include <jni.h>
#include <string.h>
#include <stdlib.h>
#include "qvp.h"

#define FN(name) JNIEXPORT JNICALL Java_net_quranpedia_qvp_QvpNative_##name
#define PG(h) ((QvpPage*)(intptr_t)(h))

static jfloatArray floats(JNIEnv* env, const float* v, jsize n) {
    jfloatArray a = (*env)->NewFloatArray(env, n);
    (*env)->SetFloatArrayRegion(env, a, 0, n, v);
    return a;
}
static jintArray ints(JNIEnv* env, const jint* v, jsize n) {
    jintArray a = (*env)->NewIntArray(env, n);
    (*env)->SetIntArrayRegion(env, a, 0, n, v);
    return a;
}
static jstring utf8(JNIEnv* env, const uint8_t* p, uint32_t n) {
    char* s = (char*)malloc(n + 1);
    memcpy(s, p, n); s[n] = 0;
    jstring r = (*env)->NewStringUTF(env, s);
    free(s);
    return r;
}

jlong FN(pageLoad)(JNIEnv* env, jclass c, jbyteArray bytes) {
    jsize n = (*env)->GetArrayLength(env, bytes);
    jbyte* p = (*env)->GetByteArrayElements(env, bytes, NULL);
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
jbyteArray FN(geomOps)(JNIEnv* env, jclass c, jlong h) {
    QvpGeometry g; qvp_geometry(PG(h), &g);
    jbyteArray a = (*env)->NewByteArray(env, g.ops_len);
    (*env)->SetByteArrayRegion(env, a, 0, g.ops_len, (const jbyte*)g.ops);
    return a;
}
jfloatArray FN(geomPts)(JNIEnv* env, jclass c, jlong h) {
    QvpGeometry g; qvp_geometry(PG(h), &g);
    return floats(env, g.pts, g.pts_len);
}
jintArray FN(geomTable)(JNIEnv* env, jclass c, jlong h) {
    QvpGeometry g; qvp_geometry(PG(h), &g);
    return ints(env, (const jint*)g.table, g.n_paths * 8);
}
jfloatArray FN(wordInfo)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpWordInfo w; if (!qvp_word_info(PG(h), idx, &w)) return NULL;
    float v[11] = { w.sura, w.ayah, w.word, w.line_no, (float)w.ayah_idx, w.x0, w.y0, w.x1, w.y1, (float)w.first_path, (float)w.n_paths };
    return floats(env, v, 11);
}
jstring FN(wordText)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpWordInfo w; if (!qvp_word_info(PG(h), idx, &w)) return NULL;
    return utf8(env, w.text, w.text_len);
}
jfloatArray FN(ayahInfo)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpAyahInfo a; if (!qvp_ayah_info(PG(h), idx, &a)) return NULL;
    float v[12] = { a.sura, a.ayah, a.part, a.parts, a.flags, (float)a.first_word, (float)a.n_words, a.marker_deco == QVP_NONE ? -1.f : (float)a.marker_deco, a.x0, a.y0, a.x1, a.y1 };
    return floats(env, v, 12);
}
jfloatArray FN(lineInfo)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpLineInfo l; if (!qvp_line_info(PG(h), idx, &l)) return NULL;
    float v[7] = { l.line_no, (float)l.first_word, (float)l.n_words, l.x0, l.y0, l.x1, l.y1 };
    return floats(env, v, 7);
}
jfloatArray FN(decoInfo)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpDecoInfo d; if (!qvp_deco_info(PG(h), idx, &d)) return NULL;
    float v[9] = { d.kind, d.sura, d.ayah, d.x0, d.y0, d.x1, d.y1, (float)d.first_path, (float)d.n_paths };
    return floats(env, v, 9);
}
jstring FN(decoText)(JNIEnv* env, jclass c, jlong h, jint idx) {
    QvpDecoInfo d; if (!qvp_deco_info(PG(h), idx, &d)) return NULL;
    return utf8(env, d.text, d.text_len);
}
static jintArray hit_out(JNIEnv* env, int ok, QvpHit* hit) {
    if (!ok) return NULL;
    jint v[3] = { hit->word == QVP_NONE ? -1 : (jint)hit->word, hit->path == QVP_NONE ? -1 : (jint)hit->path, hit->deco == QVP_NONE ? -1 : (jint)hit->deco };
    return ints(env, v, 3);
}
jintArray FN(hitTest)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y) {
    QvpHit hit; return hit_out(env, qvp_hit_test(PG(h), x, y, &hit), &hit);
}
jintArray FN(hitTestView)(JNIEnv* env, jclass c, jlong h, jfloat x, jfloat y) {
    QvpHit hit; return hit_out(env, qvp_hit_test_view(PG(h), x, y, &hit), &hit);
}
jint FN(findWord)(JNIEnv* env, jclass c, jlong h, jint s, jint a, jint w) { return qvp_find_word(PG(h), (uint16_t)s, (uint16_t)a, (uint16_t)w); }

jfloatArray FN(layout)(JNIEnv* env, jclass c, jlong h, jfloat vw, jfloat vh, jfloat pt, jfloat pb, jfloat pl, jfloat pr, jfloat spacing, jboolean fill, jint nominal) {
    QvpLayoutSpec s = { vw, vh, pt, pb, pl, pr, spacing, fill ? 1u : 0u, (uint32_t)nominal };
    QvpLayout l; qvp_layout(PG(h), &s, &l);
    jsize n = 6 + l.n_lines * 3;
    float* v = (float*)malloc(n * sizeof(float));
    v[0] = l.scale; v[1] = l.ox; v[2] = l.oy; v[3] = l.content_h; v[4] = l.pitch; v[5] = (float)l.n_lines;
    memcpy(v + 6, l.lines, l.n_lines * 3 * sizeof(float));
    jfloatArray a = floats(env, v, n);
    free(v);
    return a;
}
jint FN(style)(JNIEnv* env, jclass c, jlong h, jint sel, jint a, jint b, jint cc, jint rgba, jboolean on) {
    return qvp_style(PG(h), (uint8_t)sel, (uint32_t)a, (uint32_t)b, (uint32_t)cc, (uint32_t)rgba, on ? 1 : 0);
}
void FN(styleClear)(JNIEnv* env, jclass c, jlong h) { qvp_style_clear(PG(h)); }
void FN(styleDefault)(JNIEnv* env, jclass c, jlong h, jint rgba) { qvp_style_default(PG(h), (uint32_t)rgba); }
jintArray FN(paint)(JNIEnv* env, jclass c, jlong h) {
    QvpPageInfo i; qvp_page_info(PG(h), &i);
    const uint32_t* col = qvp_paint(PG(h));
    return ints(env, (const jint*)col, i.n_paths);
}
jintArray FN(styled)(JNIEnv* env, jclass c, jlong h) {
    uint32_t n = qvp_styled(PG(h), NULL, 0);
    if (!n) return ints(env, NULL, 0);
    uint32_t* buf = (uint32_t*)malloc(n * 8);
    qvp_styled(PG(h), buf, n);
    jintArray a = ints(env, (const jint*)buf, n * 2);
    free(buf);
    return a;
}
jstring FN(markName)(JNIEnv* env, jclass c, jint m) { return utf8(env, (const uint8_t*)qvp_mark_name((uint8_t)m), qvp_mark_name_len((uint8_t)m)); }
jstring FN(familyName)(JNIEnv* env, jclass c, jint f) { return utf8(env, (const uint8_t*)qvp_family_name((uint8_t)f), qvp_family_name_len((uint8_t)f)); }
jstring FN(kindName)(JNIEnv* env, jclass c, jint k) { return utf8(env, (const uint8_t*)qvp_kind_name((uint8_t)k), qvp_kind_name_len((uint8_t)k)); }
jint FN(version)(JNIEnv* env, jclass c) { return (jint)qvp_version(); }
