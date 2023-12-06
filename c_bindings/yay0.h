#ifndef CRUNCH64_YAY0_H
#define CRUNCH64_YAY0_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/**
 * @brief Get a size big enough to allocate a buffer that can fit the uncompressed data produced by uncompressing `src`.
 *
 * The compressed data must include the Yay0 header.
 *
 * Returning `true` means the function succeeded and the requested size was put in `dst_size`.
 *
 * If this function fails to calculate said size then it will return `false` and `dst_size` may have garbage data.
 *
 * @param dst_size[out] Will be set to the requested size.
 * @param src_len Size of `src`
 * @param src[in] Compressed Yay0 data
 */
bool crunch64_decompress_yay0_bound(size_t *dst_size, size_t src_len, const uint8_t *const src);

/**
 * @brief Decompresses the data pointed by `src` and puts that data into `dst`.
 *
 * The `dst` should point to a buffer big enough to hold the decompressed data. To know how big said buffer must be
 * refer to `crunch64_decompress_yay0_bound`.
 *
 * When this function is called, `dst_len` must point to the size of the `dst` pointer, allowing for range checking
 * and avoiding to write out of bounds.
 *
 * If the function succeedes it returns `true` and it puts the decompressed data on `dst` and the actual decompressed
 * size is put on `dst_len`.
 *
 * If this function fails it will return `false`. `dst_size` and `dst` may have garbage data.
 *
 * @param dst_len[in,out] Will be set to the decompressed size. It should point to the size of the `dst` buffer when the function is called.
 * @param dst[out] Pointer to buffer big enough to hold the decompressed data.
 * @param src_len The length of the data pointed by `src`.
 * @param src[in] Pointer to compressed data. Must contain the Yay0 header.
 */
bool crunch64_decompress_yay0(size_t *dst_len, uint8_t *dst, size_t src_len, const uint8_t *const src);

/**
 * @brief Get a size big enough to allocate a buffer that can fit the compressed data produced by compressing `src`.
 *
 * The compressed data must include the Yay0 header.
 *
 * Returning `true` means the function succeeded and the requested size was put in `dst_size`
 *
 * If this function fails to calculate said size then it will return `false` and `dst_size` may not be a valid value.
 *
 * @param dst_size[out] Will be set to the requested size.
 * @param src_len Size of `src`
 * @param src[in] Data that would be compressed
 */
bool crunch64_compress_yay0_bound(size_t *dst_size, size_t src_len, const uint8_t *const src);

/**
 * @brief Compresses the data pointed by `src` and puts that data into `dst`.
 *
 * The `dst` should point to a buffer big enough to hold the compressed data. To know how big said buffer must be
 * refer to `crunch64_compress_yay0_bound`.
 *
 * When this function is called, `dst_len` must point to the size of the `dst` pointer, allowing for range checking
 * and avoiding to write out of bounds.
 *
 * If the function succeedes it returns `true` and it puts the compressed data on `dst` and the actual compressed
 * size is put on `dst_len`.
 *
 * If this function fails it will return `false`. `dst_size` and `dst` may have garbage data.
 *
 * `dst` will include the Yay0 header.
 *
 * @param dst_len[in,out] Will be set to the compressed size. It should point to the size of the `dst` buffer when the function is called.
 * @param dst[out] Pointer to buffer big enough to hold the compressed data.
 * @param src_len The length of the data pointed by `src`.
 * @param src[in] Pointer to the decompressed data.
 */
bool crunch64_compress_yay0(size_t *dst_len, uint8_t *dst, size_t src_len, const uint8_t *const src);

#endif
