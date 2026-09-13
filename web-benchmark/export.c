#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "qvp.h"

static QvpPage *load_page(const char *folder, unsigned number) {
    char path[4096];
    snprintf(path, sizeof(path), "%s/%03u.qvp", folder, number);
    FILE *file = fopen(path, "rb");
    if (!file) { perror(path); exit(1); }
    if (fseek(file, 0, SEEK_END)) { perror(path); exit(1); }
    long size = ftell(file);
    if (size <= 0) { fprintf(stderr, "Invalid file size: %s\n", path); exit(1); }
    rewind(file);
    uint8_t *bytes = malloc(size);
    if (!bytes) { perror("malloc"); exit(1); }
    if (fread(bytes, 1, size, file) != (size_t)size) exit(1);
    fclose(file);
    QvpPage *page = qvp_page_load(bytes, size);
    free(bytes);
    if (!page) { fprintf(stderr, "Cannot decode %s\n", path); exit(1); }
    return page;
}

static void export_page(const char *folder, const char *output, unsigned number) {
    QvpPage *page = load_page(folder, number);
    QvpPageInfo info;
    QvpGeometry geometry;
    qvp_page_info(page, &info);
    qvp_geometry(page, &geometry);
    char path[4096];
    snprintf(path, sizeof(path), "%s/%03u.bin", output, number);
    FILE *file = fopen(path, "wb");
    if (!file) { perror(path); exit(1); }
    fwrite("QVB1", 1, 4, file);
    fwrite(&info.width, 4, 1, file);
    fwrite(&info.height, 4, 1, file);
    uint32_t counts[] = {geometry.n_paths, geometry.ops_len, geometry.pts_len, info.n_words, number};
    fwrite(counts, 4, 5, file);
    fwrite(geometry.table, 4, geometry.n_paths * 8, file);
    fwrite(geometry.ops, 1, geometry.ops_len, file);
    uint32_t zero = 0;
    fwrite(&zero, 1, (4 - geometry.ops_len % 4) % 4, file);
    fwrite(geometry.pts, 4, geometry.pts_len, file);
    for (unsigned i = 0; i < info.n_words; i++) {
        QvpWordInfo word;
        if (!qvp_word_info(page, i, &word)) exit(1);
        float box[] = {word.x0, word.y0, word.x1, word.y1};
        fwrite(box, 4, 4, file);
    }
    printf("page %03u: %u paths, %u coordinate floats, %ld bytes\n", number, geometry.n_paths, geometry.pts_len, ftell(file));
    if (ferror(file) || fclose(file)) { fprintf(stderr, "Could not write %s\n", path); exit(1); }
    qvp_page_free(page);
}

int main(int argc, char **argv) {
    if (argc < 3) return 1;
    if (argc > 3) {
        for (int i = 3; i < argc; i++) {
            char *end;
            long number = strtol(argv[i], &end, 10);
            if (!*argv[i] || *end || number < 1 || number > 604) {
                fprintf(stderr, "Expected page number 1–604: %s\n", argv[i]);
                return 1;
            }
        }
        for (int i = 3; i < argc; i++) export_page(argv[1], argv[2], (unsigned)strtol(argv[i], NULL, 10));
        return 0;
    }
    unsigned densest = 1, max_points = 0;
    for (unsigned number = 1; number <= 604; number++) {
        QvpPage *page = load_page(argv[1], number);
        QvpGeometry geometry;
        qvp_geometry(page, &geometry);
        if (geometry.pts_len > max_points) { max_points = geometry.pts_len; densest = number; }
        qvp_page_free(page);
    }
    unsigned pages[] = {1, 42, densest};
    for (unsigned i = 0; i < 3; i++) export_page(argv[1], argv[2], pages[i]);
    char path[4096];
    snprintf(path, sizeof(path), "%s/manifest.json", argv[2]);
    FILE *file = fopen(path, "w");
    if (!file) return 1;
    QvpStr engine_version;
    qvp_version(&engine_version);
    fprintf(file, "{\"format\":\"QVB1\",\"engine_name\":\"%s\",\"engine_version\":\"%.*s\",\"pages\":[1,42,%u],\"densest_page\":%u,\"selection\":\"Opening page, explicitly selected example page 42, largest coordinate count across all 604 pages; not a statistical sample\",\"metrics\":{", qvp_engine_name(), (int)engine_version.len, (const char *)engine_version.ptr, densest, densest);
    for (unsigned i = 0; i < 3; i++) {
        QvpPage *page = load_page(argv[1], pages[i]);
        QvpGeometry geometry;
        qvp_geometry(page, &geometry);
        fprintf(file, "%s\"%u\":{\"paths\":%u,\"operations\":%u,\"coordinate_floats\":%u}", i ? "," : "", pages[i], geometry.n_paths, geometry.ops_len, geometry.pts_len);
        qvp_page_free(page);
    }
    fprintf(file, "}}\n");
    return ferror(file) || fclose(file) != 0;
}
