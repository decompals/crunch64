#include "crunch64.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

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

    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);

    uint8_t *data = malloc(*size * sizeof(uint8_t));
    fread(data, sizeof(uint8_t), *size, f);
    fclose(f);
    return data;
}

void write_binary_file(const char *path, uint8_t *data, size_t size)
{
    assert(path != NULL);
    assert(data != NULL);

    FILE *f = fopen(path, "wb");
    fwrite(data, sizeof(uint8_t), size, f);
    fclose(f);
}

int main(void)
{
    int ret = EXIT_SUCCESS;

    size_t compressed_size;
    uint8_t *compressed_data = read_binary_file("test_data/small.txt.Yaz0", &compressed_size);

    size_t decompressed_size;
    uint8_t *decompressed_data = NULL;

    bool size_request_ok = crunch64_decompress_yaz0_get_dst_buffer_size(&decompressed_size, compressed_size, compressed_data);
    if (!size_request_ok)
    {
        fprintf(stderr, "failed to request size for buffer\n");
        goto failure;
    }

    decompressed_data = malloc(decompressed_size * sizeof(uint8_t));

    bool decompress_ok = crunch64_decompress_yaz0(&decompressed_size, decompressed_data, compressed_size, compressed_data);
    if (!decompress_ok)
    {
        fprintf(stderr, "failed to decompress file\n");
        goto failure;
    }

    write_binary_file("small.txt", decompressed_data, decompressed_size);

    if (0)
    {
    failure:
        ret = EXIT_FAILURE;
    }
    FREE(compressed_data);
    FREE(decompressed_data);

    return ret;
}
