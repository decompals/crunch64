#include "utils.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

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
