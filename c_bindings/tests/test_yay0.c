#include "crunch64.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

#include "utils.h"

bool decompress(size_t *dst_size, uint8_t **dst, size_t src_size, const uint8_t *src)
{
    size_t decompressed_size;
    uint8_t *decompressed_data = NULL;

    Crunch64Error size_request_ok = crunch64_decompress_yay0_bound(&decompressed_size, src_size, src);
    if (size_request_ok != Crunch64Error_Okay)
    {
        fprintf(stderr, " failed to request size for buffer. Reason: %s\n", get_crunch64_error_str(size_request_ok));
        return false;
    }

    decompressed_data = malloc(decompressed_size * sizeof(uint8_t));
    if (decompressed_data == NULL)
    {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", decompressed_size * sizeof(uint8_t));
        return false;
    }

    Crunch64Error decompress_ok = crunch64_decompress_yay0(&decompressed_size, decompressed_data, src_size, src);
    if (decompress_ok != Crunch64Error_Okay)
    {
        fprintf(stderr, " failed to decompress data. Reason: %s\n", get_crunch64_error_str(decompress_ok));
        free(decompressed_data);
        return false;
    }

    *dst_size = decompressed_size;
    *dst = decompressed_data;

    fprintf(stderr, " OK\n");
    return true;
}

bool compress(size_t *dst_size, uint8_t **dst, size_t src_size, const uint8_t *src)
{
    size_t compressed_size;
    uint8_t *compressed_data = NULL;

    assert(dst_size != NULL);
    assert(dst != NULL);
    assert(src != NULL);

    Crunch64Error size_request_ok = crunch64_compress_yay0_bound(&compressed_size, src_size, src);
    if (size_request_ok != Crunch64Error_Okay)
    {
        fprintf(stderr, " failed to request size for buffer. Reason: %s\n", get_crunch64_error_str(size_request_ok));
        return false;
    }

    compressed_data = malloc(compressed_size * sizeof(uint8_t));
    if (compressed_data == NULL)
    {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", compressed_size * sizeof(uint8_t));
        return false;
    }

    Crunch64Error compress_ok = crunch64_compress_yay0(&compressed_size, compressed_data, src_size, src);
    if (compress_ok != Crunch64Error_Okay)
    {
        fprintf(stderr, " failed to decompress data. Reason: %s\n", get_crunch64_error_str(compress_ok));
        free(compressed_data);
        return false;
    }

    *dst_size = compressed_size;
    *dst = compressed_data;

    fprintf(stderr, " OK\n");

    return true;
}

void print_usage(int argc, char *argv[])
{
    (void)argc;

    fprintf(stderr, "Usage: %s bin_file compressed_file\n", argv[0]);
    fprintf(stderr, "\n");
    fprintf(stderr, "This programs tests compression and decompression produces matching output\n");
}

int main(int argc, char *argv[])
{
    int ret = 0;

    if (argc < 2)
    {
        print_usage(argc, argv);
        return -1;
    }

    const char *bin_path = argv[1];
    const char *compressed_path = argv[2];

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

    if (!test_matching_decompression(bin_size, bin, compressed_data_size, compressed_data))
    {
        ret++;
    }
    if (!test_matching_compression(bin_size, bin, compressed_data_size, compressed_data))
    {
        ret++;
    }
    if (!test_cycle_decompressed(bin_size, bin))
    {
        ret++;
    }
    if (!test_cycle_compressed(compressed_data_size, compressed_data))
    {
        ret++;
    }

    free(bin);
    free(compressed_data);

    return ret;
}
