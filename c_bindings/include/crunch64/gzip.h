#ifndef CRUNCH64_GZIP_H
#define CRUNCH64_GZIP_H
#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "error.h"

#ifdef __cplusplus
extern "C"
{
#endif

/**
 * @brief Get a size big enough to allocate a buffer that can fit the compressed data produced by compressing `src`.
 *
 * Returning `true` means the function succeeded and the requested size was put in `dst_size`
 *
 * If this function fails to calculate said size then it will return `false` and `dst_size` may not be a valid value.
 *
 * @param dst_size[out] Will be set to the requested size.
 * @param src_len Size of `src`
 * @param src[in] Data that would be compressed
 */
Crunch64Error crunch64_gzip_compress_bound(size_t *dst_size, size_t src_len, const uint8_t *const src);

/**
 * @brief Compresses the data pointed by `src` and puts that data into `dst`.
 *
 * The `dst` should point to a buffer big enough to hold the compressed data. To know how big said buffer must be
 * refer to `crunch64_gzip_compress_bound`.
 *
 * When this function is called, `dst_len` must point to the size of the `dst` pointer, allowing for range checking
 * and avoiding to write out of bounds.
 *
 * If the function succeedes it returns `true` and it puts the compressed data on `dst` and the actual compressed
 * size is put on `dst_len`.
 *
 * If this function fails it will return `false`. `dst_size` and `dst` may have garbage data.
 *
 * `dst` will include the gzip footer but no gzip header.
 *
 * @param dst_len[in,out] Will be set to the compressed size. It should point to the size of the `dst` buffer when the function is called.
 * @param dst[out] Pointer to buffer big enough to hold the compressed data.
 * @param src_len The length of the data pointed by `src`.
 * @param src[in] Pointer to the decompressed data.
 * @param level Compression level (4-9).
 * @param small_mem If `true` then the function will output compressed blocks more often.
 */
Crunch64Error crunch64_gzip_compress(size_t *dst_len, uint8_t *dst, size_t src_len, const uint8_t *const src, int level, bool small_mem);

#ifdef __cplusplus
}
#endif

#endif
