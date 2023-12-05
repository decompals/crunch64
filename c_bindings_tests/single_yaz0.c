#include "crunch64.h"

#include <stdio.h>
#include <stdlib.h>

int main(void)
{
    FILE *compressed_file = fopen("test_data/small.txt.Yaz0", "rb");

    fseek(compressed_file, 0, SEEK_END);
    size_t compressed_size = ftell(compressed_file);
    fseek(compressed_file, 0, SEEK_SET);

    uint8_t *compressed_data = malloc(compressed_size * sizeof(uint8_t));
    fread(compressed_data, sizeof(uint8_t), compressed_size, compressed_file);
    fclose(compressed_file);

    uint8_t *decompressed_data = malloc(0x200 * sizeof(uint8_t));
    size_t decompressed_size;

    bool decompress_ok = crunch64_decompress_yaz0(&decompressed_size, decompressed_data, compressed_size, compressed_data);

    free(compressed_data);

    if (!decompress_ok)
    {
        fprintf(stderr, "failed to decompress file\n");
        free(decompressed_data);
        return 1;
    }

    FILE *decompressed_file = fopen("small.txt", "wb");
    fwrite(decompressed_data, sizeof(uint8_t), decompressed_size, decompressed_file);
    fclose(decompressed_file);

    free(decompressed_data);

    return 0;
}
