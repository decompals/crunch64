#ifndef UTILS_H
#define UTILS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "crunch64.h"

uint8_t *read_binary_file(const char *path, size_t *size);
bool write_binary_file(const char *path, uint8_t *data, size_t size);

bool compare_buffers(size_t a_size, const uint8_t *a, size_t b_size, const uint8_t *b);

bool test_matching_decompression(size_t bin_size, uint8_t *bin, size_t compressed_data_size, uint8_t *compressed_data);
bool test_matching_compression(size_t bin_size, uint8_t *bin, size_t uncompressed_data_size, uint8_t *uncompressed_data);

bool test_cycle_decompressed(size_t bin_size, uint8_t *bin);
bool test_cycle_compressed(size_t compressed_data_size, uint8_t *compressed_data);

// These should be provided by the specific test C file
bool decompress(size_t *dst_size, uint8_t **dst, size_t src_size, const uint8_t *src);
bool compress(size_t *dst_size, uint8_t **dst, size_t src_size, const uint8_t *src);

const char *get_crunch64_error_str(Crunch64Error error);

#endif
