#ifndef CRUNCH64_H
#define CRUNCH64_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

bool crunch64_decompress_yaz0(size_t *dst_len, uint8_t *dst, size_t src_len, const uint8_t *const src);

#endif
