#include "crunch64.h"

#include <assert.h>
#include <dirent.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef Crunch64Error (*compress_bound_fn)(size_t *dst_size, size_t src_len, const uint8_t *const src);
typedef Crunch64Error (*compress_fn)(size_t *dst_size, uint8_t *dst, size_t src_size, const uint8_t *src);

const char *const crunch64_error_str[] = {
    [Crunch64Error_Okay] = "Okay",
    [Crunch64Error_InvalidYay0Header] = "Invalid Yay0 header",
    [Crunch64Error_InvalidYaz0Header] = "Invalid Yaz0 header",
    [Crunch64Error_InvalidMio0Header] = "Invalid Mio0 header",
    [Crunch64Error_UnsupportedCompressionType] = "Unsupported compression type",
    [Crunch64Error_UnalignedRead] = "Unaligned read",
    [Crunch64Error_ByteConversion] = "Byte conversion",
    [Crunch64Error_OutOfBounds] = "Out of bounds",
    [Crunch64Error_NullPointer] = "Null pointer",
};

const char *get_crunch64_error_str(Crunch64Error error) {
    return crunch64_error_str[error];
}

bool has_suffix(const char *str, const char *suffix) {
    size_t str_len = strlen(str);
    size_t suffix_len = strlen(suffix);
    return str_len >= suffix_len && strcmp(str + str_len - suffix_len, suffix) == 0;
}

uint8_t *read_binary_file(const char *path, size_t *size) {
    assert(path != NULL);
    assert(size != NULL);

    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return NULL;
    }

    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);

    uint8_t *data = malloc(*size * sizeof(uint8_t));
    if (data == NULL) {
        fclose(f);
        return NULL;
    }

    size_t count = fread(data, sizeof(uint8_t), *size, f);
    if (count != *size) {
        free(data);
        fclose(f);
        return NULL;
    }

    fclose(f);
    return data;
}

bool write_binary_file(const char *path, uint8_t *data, size_t size) {
    assert(path != NULL);
    assert(data != NULL);

    FILE *f = fopen(path, "wb");
    if (f == NULL) {
        return false;
    }

    size_t count = fwrite(data, sizeof(uint8_t), size, f);
    if (count != size) {
        return false;
    }

    fclose(f);

    return true;
}

bool compare_buffers(size_t a_size, const uint8_t *a, size_t b_size, const uint8_t *b) {
    if (a_size != b_size) {
        fprintf(stderr, " sizes don't match\n");
        return false;
    }

    if (memcmp(a, b, a_size) != 0) {
        fprintf(stderr, " data doesn't match\n");
        return false;
    }

    fprintf(stderr, " OK\n");

    return true;
}

bool test_matching_decompression(compress_bound_fn decompress_bound, compress_fn decompress, size_t bin_size,
                                 uint8_t *bin, size_t compressed_size, uint8_t *compressed_data) {
    fprintf(stderr, "Testing matching decompression:\n");

    fprintf(stderr, "    decompressing: ");
    size_t decompressed_size;
    uint8_t *decompressed_data = NULL;

    Crunch64Error size_request_ok = decompress_bound(&decompressed_size, compressed_size, compressed_data);
    if (size_request_ok != Crunch64Error_Okay) {
        fprintf(stderr, " failed to request size for buffer. Reason: %s\n", get_crunch64_error_str(size_request_ok));
        return false;
    }

    decompressed_data = malloc(decompressed_size * sizeof(uint8_t));
    if (decompressed_data == NULL) {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", decompressed_size * sizeof(uint8_t));
        return false;
    }

    Crunch64Error decompress_ok = decompress(&decompressed_size, decompressed_data, compressed_size, compressed_data);
    if (decompress_ok != Crunch64Error_Okay) {
        fprintf(stderr, " failed to decompress data. Reason: %s\n", get_crunch64_error_str(decompress_ok));
        free(decompressed_data);
        return false;
    }

    fprintf(stderr, " OK\n");

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(decompressed_size, decompressed_data, bin_size, bin);
    if (!matches) {
        free(decompressed_data);
        return false;
    }

    free(decompressed_data);
    return true;
}

bool test_matching_compression(compress_bound_fn compress_bound, compress_fn compress, size_t bin_size, uint8_t *bin,
                               size_t compressed_size, uint8_t *compressed_data) {
    fprintf(stderr, "Testing matching compression:\n");
    fprintf(stderr, "    compressing: ");

    size_t recompressed_size;
    uint8_t *recompressed_data = NULL;

    Crunch64Error size_request_ok = compress_bound(&recompressed_size, bin_size, bin);
    if (size_request_ok != Crunch64Error_Okay) {
        fprintf(stderr, " failed to request size for buffer. Reason: %s\n", get_crunch64_error_str(size_request_ok));
        return false;
    }

    recompressed_data = malloc(recompressed_size * sizeof(uint8_t));
    if (recompressed_data == NULL) {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", recompressed_size * sizeof(uint8_t));
        return false;
    }

    Crunch64Error compress_ok = compress(&recompressed_size, recompressed_data, bin_size, bin);
    if (compress_ok != Crunch64Error_Okay) {
        fprintf(stderr, " failed to decompress data. Reason: %s\n", get_crunch64_error_str(compress_ok));
        free(recompressed_data);
        return false;
    }

    fprintf(stderr, " OK\n");

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(recompressed_size, recompressed_data, compressed_size, compressed_data);
    if (!matches) {
        free(recompressed_data);
        return false;
    }

    free(recompressed_data);
    return true;
}

int errors = 0;

void run_tests(const char *name, const char *file_extension, compress_bound_fn compress_bound, compress_fn compress,
               compress_bound_fn decompress_bound, compress_fn decompress) {
    struct dirent *entry;
    DIR *dir = opendir("test_data");
    if (!dir) {
        fprintf(stderr, "Could not open test_data directory\n");
        errors++;
        return;
    }

    fprintf(stderr, "Running tests for %s\n", name);
    fprintf(stderr, "\n");

    bool found_tests = false;
    while ((entry = readdir(dir)) != NULL) {
        if (!has_suffix(entry->d_name, file_extension)) {
            continue;
        }

        found_tests = true;

        char bin_path[512];
        snprintf(bin_path, sizeof(bin_path), "test_data/%s", entry->d_name);
        bin_path[strlen(bin_path) - strlen(file_extension)] = '\0'; // remove file extension

        char compressed_path[512];
        snprintf(compressed_path, sizeof(compressed_path), "test_data/%s", entry->d_name);

        fprintf(stderr, "Reading file %s\n", bin_path);
        size_t bin_size = 0;
        uint8_t *bin = read_binary_file(bin_path, &bin_size);
        assert(bin_size > 0);
        assert(bin != NULL);

        fprintf(stderr, "Reading file %s\n", compressed_path);
        size_t compressed_data_size = 0;
        uint8_t *compressed_data = read_binary_file(compressed_path, &compressed_data_size);
        assert(compressed_data_size > 0);
        assert(compressed_data != NULL);

        if (!test_matching_decompression(decompress_bound, decompress, bin_size, bin, compressed_data_size, compressed_data)) {
            errors++;
        }
        if (!test_matching_compression(compress_bound, compress, bin_size, bin, compressed_data_size, compressed_data)) {
            errors++;
        }

        fprintf(stderr, "\n");

        free(bin);
        free(compressed_data);
    }

    if (!found_tests) {
        fprintf(stderr, "No test files found for %s\n", name);
        errors++;
        return;
    }
}

int main(void) {
    run_tests("yay0", ".Yay0", crunch64_yay0_compress_bound, crunch64_yay0_compress, crunch64_yay0_decompress_bound, crunch64_yay0_decompress);
    run_tests("yaz0", ".Yaz0", crunch64_yaz0_compress_bound, crunch64_yaz0_compress, crunch64_yaz0_decompress_bound, crunch64_yaz0_decompress);
    run_tests("mio0", ".MIO0", crunch64_mio0_compress_bound, crunch64_mio0_compress, crunch64_mio0_decompress_bound, crunch64_mio0_decompress);

    if (errors == 0) {
        fprintf(stderr, "All tests passed\n");
        return 0;
    } else {
        fprintf(stderr, "%d tests failed\n", errors);
        return 1;
    }
}
