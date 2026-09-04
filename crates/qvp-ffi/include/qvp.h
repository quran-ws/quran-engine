/* QVP engine C ABI — generated to match crates/qvp-ffi/src/lib.rs (keep in sync). */
#ifndef QVP_H
#define QVP_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct QvpPage QvpPage;

typedef struct { float width, height; uint32_t page, n_lines, n_ayahs, n_words, n_paths, n_decos; } QvpPageInfo;
/* table: n_paths × 6 uint32: op_start, op_count, pt_start, pt_count, flags, word.
   flags = kind | mark<<8 | family<<16 | (evenodd?1:0)<<24 | (glyph_instance?2:0)<<24
   ops: 0=MoveTo 1=LineTo 2=QuadTo 3=CubicTo 4=Close ; pts: x,y pairs consumed in order. */
typedef struct { const uint8_t* ops; uint32_t ops_len; const float* pts; uint32_t pts_len; const uint32_t* table; uint32_t n_paths; } QvpGeometry;
typedef struct { uint16_t sura, ayah, word, line_no; uint32_t ayah_idx; float x0, y0, x1, y1; const uint8_t* text; uint32_t text_len; uint32_t first_path, n_paths; } QvpWordInfo;
typedef struct { uint16_t sura, ayah; uint8_t part, parts, flags; uint32_t first_word, n_words, marker_deco; float x0, y0, x1, y1; } QvpAyahInfo;
typedef struct { uint8_t line_no; uint32_t first_word, n_words; float x0, y0, x1, y1; } QvpLineInfo;
typedef struct { uint8_t kind; uint16_t sura, ayah; float x0, y0, x1, y1; const uint8_t* text; uint32_t text_len; uint32_t first_path, n_paths; } QvpDecoInfo;
typedef struct { uint32_t word, path, deco; } QvpHit;   /* 0xFFFFFFFF = none */

#define QVP_NONE 0xFFFFFFFFu
enum { QVP_SEL_PATH = 0, QVP_SEL_WORD, QVP_SEL_AYAH, QVP_SEL_LINE, QVP_SEL_MARK, QVP_SEL_FAMILY, QVP_SEL_KIND, QVP_SEL_DECO };
enum { QVP_KIND_BODY = 0, QVP_KIND_MARK, QVP_KIND_AYAH_NUMBER, QVP_KIND_AYAH_ORNAMENT, QVP_KIND_HEADER_INK, QVP_KIND_OTHER = 255 };
enum { QVP_FAMILY_NONE = 0, QVP_FAMILY_DIACRITIC, QVP_FAMILY_TANWEEN, QVP_FAMILY_DOTS, QVP_FAMILY_WAQF, QVP_FAMILY_SIFR, QVP_FAMILY_SAJDAH, QVP_FAMILY_READING_SIGN };
enum { QVP_DECO_AYAH_MARKER = 0, QVP_DECO_SURAH_NAME, QVP_DECO_BASMALAH, QVP_DECO_HIZB_MARK, QVP_DECO_SAJDAH_MARK };

uint8_t* qvp_alloc(size_t len);
void     qvp_dealloc(uint8_t* p, size_t len);

QvpPage* qvp_page_load(const uint8_t* bytes, size_t len);      /* NULL on error; bytes are copied */
void     qvp_page_free(QvpPage* page);
void     qvp_page_info(const QvpPage* page, QvpPageInfo* out);
void     qvp_geometry(const QvpPage* page, QvpGeometry* out);   /* pointers live as long as the page */
int      qvp_word_info(const QvpPage* page, uint32_t idx, QvpWordInfo* out);
int      qvp_ayah_info(const QvpPage* page, uint32_t idx, QvpAyahInfo* out);
int      qvp_line_info(const QvpPage* page, uint32_t idx, QvpLineInfo* out);
int      qvp_deco_info(const QvpPage* page, uint32_t idx, QvpDecoInfo* out);

int      qvp_hit_test(const QvpPage* page, float x, float y, QvpHit* out);   /* x,y in page units */
int32_t  qvp_find_word(const QvpPage* page, uint16_t sura, uint16_t ayah, uint16_t word);

int      qvp_style(QvpPage* page, uint8_t sel_kind, uint32_t a, uint32_t b, uint32_t c, uint32_t rgba, int on);
void     qvp_style_clear(QvpPage* page);
void     qvp_style_default(QvpPage* page, uint32_t rgba);
const uint32_t* qvp_paint(QvpPage* page);                        /* n_paths colours, 0xRRGGBBAA; alpha 0 = hidden */
uint32_t qvp_styled(const QvpPage* page, uint32_t* out_pairs, uint32_t cap);

const char* qvp_mark_name(uint8_t mark);     uint32_t qvp_mark_name_len(uint8_t mark);   /* not NUL-terminated */
const char* qvp_family_name(uint8_t f);      uint32_t qvp_family_name_len(uint8_t f);
const char* qvp_kind_name(uint8_t k);        uint32_t qvp_kind_name_len(uint8_t k);
uint32_t    qvp_version(void);

#ifdef __cplusplus
}
#endif
#endif
