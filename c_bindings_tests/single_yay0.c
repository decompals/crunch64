#include "crunch64.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FREE(ptr)      \
    if ((ptr) != NULL) \
    {                  \
        free((ptr));   \
    }

uint8_t *read_binary_file(const char *path, size_t *size)
{
    assert(path != NULL);
    assert(size != NULL);

    FILE *f = fopen(path, "rb");
    if (f == NULL)
    {
        return NULL;
    }

    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);

    uint8_t *data = malloc(*size * sizeof(uint8_t));
    if (data == NULL)
    {
        fclose(f);
        return NULL;
    }

    size_t count = fread(data, sizeof(uint8_t), *size, f);
    if (count != *size)
    {
        free(data);
        fclose(f);
        return NULL;
    }

    fclose(f);
    return data;
}

bool write_binary_file(const char *path, uint8_t *data, size_t size)
{
    assert(path != NULL);
    assert(data != NULL);

    FILE *f = fopen(path, "wb");
    if (f == NULL)
    {
        return false;
    }

    size_t count = fwrite(data, sizeof(uint8_t), size, f);
    if (count != size)
    {
        return false;
    }

    fclose(f);

    return true;
}

bool decompress(size_t *dst_size, uint8_t **dst, size_t src_size, const uint8_t *src)
{
    size_t decompressed_size;
    uint8_t *decompressed_data = NULL;

    bool size_request_ok = crunch64_decompress_yay0_bound(&decompressed_size, src_size, src);
    if (!size_request_ok)
    {
        fprintf(stderr, " failed to request size for buffer\n");
        return false;
    }

    decompressed_data = malloc(decompressed_size * sizeof(uint8_t));
    if (decompressed_data == NULL)
    {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", decompressed_size * sizeof(uint8_t));
        return false;
    }

    bool decompress_ok = crunch64_decompress_yay0(&decompressed_size, decompressed_data, src_size, src);
    if (!decompress_ok)
    {
        fprintf(stderr, " failed to decompress data\n");
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

    bool size_request_ok = crunch64_compress_yay0_bound(&compressed_size, src_size, src);
    if (!size_request_ok)
    {
        fprintf(stderr, " failed to request size for buffer\n");
        return false;
    }

    compressed_data = malloc(compressed_size * sizeof(uint8_t));
    if (compressed_data == NULL)
    {
        fprintf(stderr, " malloc fail: 0x%zX bytes\n", compressed_size * sizeof(uint8_t));
        return false;
    }

    bool compress_ok = crunch64_compress_yay0(&compressed_size, compressed_data, src_size, src);
    if (!compress_ok)
    {
        fprintf(stderr, " failed to compress data\n");
        free(compressed_data);
        return false;
    }

    *dst_size = compressed_size;
    *dst = compressed_data;

    fprintf(stderr, " OK\n");

    return true;
}

bool compare_buffers(size_t a_size, const uint8_t *a, size_t b_size, const uint8_t *b)
{
    if (a_size != b_size)
    {
        fprintf(stderr, " sizes don't match\n");
        return false;
    }

    if (memcmp(a, b, a_size) != 0)
    {
        fprintf(stderr, " data doesn't match\n");
        return false;
    }

    fprintf(stderr, " OK\n");

    return true;
}

bool test_matching_decompression(size_t bin_size, uint8_t *bin, size_t compressed_data_size, uint8_t *compressed_data)
{
    fprintf(stderr, "Testing matching decompression:\n");

    size_t buffer_size;
    uint8_t *buffer;

    fprintf(stderr, "    decompressing: ");
    bool decompress_ok = decompress(&buffer_size, &buffer, compressed_data_size, compressed_data);

    if (!decompress_ok)
    {
        return false;
    }

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(buffer_size, buffer, bin_size, bin);
    if (!matches)
    {
        free(buffer);
        return false;
    }

    free(buffer);
    return true;
}

bool test_matching_compression(size_t bin_size, uint8_t *bin, size_t uncompressed_data_size, uint8_t *uncompressed_data)
{
    fprintf(stderr, "Testing matching compression:\n");

    size_t buffer_size;
    uint8_t *buffer;

    fprintf(stderr, "    compressing: ");
    bool compress_ok = compress(&buffer_size, &buffer, bin_size, bin);

    if (!compress_ok)
    {
        return false;
    }

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(buffer_size, buffer, uncompressed_data_size, uncompressed_data);
    if (!matches)
    {
        free(buffer);
        return false;
    }

    free(buffer);
    return true;
}

bool test_cycle_decompressed(size_t bin_size, uint8_t *bin)
{
    fprintf(stderr, "Testing cycle decompression:\n");

    size_t buffer_size;
    uint8_t *buffer;
    {
        size_t temp_buffer_size;
        uint8_t *temp_buffer;

        fprintf(stderr, "    compressing: ");
        bool compress_ok = compress(&temp_buffer_size, &temp_buffer, bin_size, bin);
        if (!compress_ok)
        {
            return false;
        }

        fprintf(stderr, "    decompressing: ");
        bool decompress_ok = decompress(&buffer_size, &buffer, temp_buffer_size, temp_buffer);
        if (!decompress_ok)
        {
            free(temp_buffer);
            return false;
        }

        free(temp_buffer);
    }

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(buffer_size, buffer, bin_size, bin);
    if (!matches)
    {
        free(buffer);
        return false;
    }

    free(buffer);
    return true;
}

bool test_cycle_compressed(size_t compressed_data_size, uint8_t *compressed_data)
{
    fprintf(stderr, "Testing cycle compression:\n");

    size_t buffer_size;
    uint8_t *buffer;
    {
        size_t temp_buffer_size;
        uint8_t *temp_buffer;

        fprintf(stderr, "    decompressing: ");
        bool decompress_ok = decompress(&temp_buffer_size, &temp_buffer, compressed_data_size, compressed_data);
        if (!decompress_ok)
        {
            return false;
        }

        fprintf(stderr, "    compressing: ");
        bool compress_ok = compress(&buffer_size, &buffer, temp_buffer_size, temp_buffer);
        if (!compress_ok)
        {
            free(temp_buffer);
            return false;
        }

        free(temp_buffer);
    }

    fprintf(stderr, "    validating data: ");
    bool matches = compare_buffers(buffer_size, buffer, compressed_data_size, compressed_data);
    if (!matches)
    {
        free(buffer);
        return false;
    }

    free(buffer);
    return true;
}

#define BIN_PATH "test_data/x86-64_rabbitizer.bin"
#define COMPRESSED_PATH "test_data/x86-64_rabbitizer.bin.Yay0"

int main(void)
{
    int ret = 0;

    fprintf(stderr, "Reading file %s\n", BIN_PATH);
    size_t bin_size = 0;
    uint8_t *bin = read_binary_file(BIN_PATH, &bin_size);
    assert(bin_size > 0);
    assert(bin != NULL);

    fprintf(stderr, "Reading file %s\n", COMPRESSED_PATH);
    size_t compressed_data_size = 0;
    uint8_t *compressed_data = read_binary_file(COMPRESSED_PATH, &compressed_data_size);
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
