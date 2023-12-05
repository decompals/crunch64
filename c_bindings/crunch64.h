#ifndef CRUNCH64_H
#define CRUNCH64_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/**
 * @brief Get a size big enough to allocate a buffer that can fit the uncompressed data produced by uncompressing `src`.
 * Returning `true` means the function succeded and the requested size was put in `dst_size`
 * If this function fails to calculate said size then it will return `false` and `dst_size` may not be a valid value.
 *
 * @param dst_size[out] Will be set to the requested size.
 * @param src_len Size of `src`
 * @param src[in] Compressed Yaz0 data
 */
bool crunch64_decompress_yaz0_get_dst_buffer_size(size_t *dst_size, size_t src_len, const uint8_t *const src);

/**
 * @brief
 *
 * @param dst_len[in, out]
 * @param dst[in, out]
 * @param src_len
 * @param src[in]
 * @return true
 * @return false
 */
bool crunch64_decompress_yaz0(size_t *dst_len, uint8_t *dst, size_t src_len, const uint8_t *const src);

#endif
