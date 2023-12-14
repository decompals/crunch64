#!/usr/bin/env python3

from __future__ import annotations

import crunch64
from pathlib import Path


def test_matching_decompression(
    bin_data: bytes, comp_data: bytes
) -> bool:
    print("Testing matching decompression:")

    print("    Decompressing: ", end="")
    decompressed = crunch64.decompress_yaz0(comp_data)
    print(" OK")

    print("    Validating: ", end="")
    equal = decompressed == bin_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal

def test_matching_compression(
    bin_data: bytes, comp_data: bytes
) -> bool:
    print("Testing matching decompression:")

    print("    Compressing: ", end="")
    compressed = crunch64.compress_yaz0(bin_data)
    print(" OK")

    print("    Validating: ", end="")
    equal = compressed == comp_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal

def test_cycle_decompressed(
    bin_data: bytes
) -> bool:
    print("Testing cycle decompression:")

    print("    Compressing: ", end="")
    compressed = crunch64.compress_yaz0(bin_data)
    print(" OK")

    print("    Decompressing: ", end="")
    dec = crunch64.decompress_yaz0(compressed)
    print(" OK")

    print("    Validating: ", end="")
    equal = dec == bin_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal

def test_cycle_compressed(
    comp_data: bytes
) -> bool:
    print("Testing cycle compression:")

    print("    Decompressing: ", end="")
    dec = crunch64.decompress_yaz0(comp_data)
    print(" OK")

    print("    Compressing: ", end="")
    compressed = crunch64.compress_yaz0(dec)
    print(" OK")

    print("    Validating: ", end="")
    equal = compressed == comp_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal


errors = 0
for comp_path in Path('test_data').glob('*.yaz0'):
    bin_path = comp_path.with_suffix("")

    print(f"Reading {bin_path}")
    bin_data = bin_path.read_bytes()

    print(f"Reading {comp_path}")
    comp_data = comp_path.read_bytes()

    if not test_matching_decompression(bin_data, comp_data):
        errors += 1
    if not test_matching_compression(bin_data, comp_data):
        errors += 1
    if not test_cycle_decompressed(bin_data):
        errors += 1
    if not test_cycle_compressed(comp_data):
        errors += 1

    print()

exit(errors)
