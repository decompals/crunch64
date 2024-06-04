#!/usr/bin/env python3

from __future__ import annotations

import crunch64
from pathlib import Path
from typing import Callable


def test_matching_decompression(
    decompress: Callable[[bytes], bytes], bin_data: bytes, comp_data: bytes
) -> bool:
    print("Testing matching decompression:")

    print("    Decompressing: ", end="")
    decompressed = decompress(comp_data)
    print(" OK")

    print("    Validating: ", end="")
    equal = decompressed == bin_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal


def test_matching_compression(
    compress: Callable[[bytes], bytes], bin_data: bytes, comp_data: bytes
) -> bool:
    print("Testing matching decompression:")

    print("    Compressing: ", end="")
    compressed = compress(bin_data)
    print(" OK")

    print("    Validating: ", end="")
    equal = compressed == comp_data
    if equal:
        print(" OK")
    else:
        print(" data doesn't match")
    return equal


errors = 0


def run_tests(
    name: str,
    file_extension: str,
    compress: Callable[[bytes], bytes],
    decompress: Callable[[bytes], bytes],
):
    global errors

    comp_paths = list(sorted(Path("test_data").glob(f"*{file_extension}")))
    if not comp_paths:
        print(f"No test files found for {name}")
        errors += 1
        return

    print(f"Testing {name}")
    print()

    for comp_path in comp_paths:
        bin_path = comp_path.with_suffix("")

        print(f"Reading {bin_path}")
        bin_data = bin_path.read_bytes()

        print(f"Reading {comp_path}")
        comp_data = comp_path.read_bytes()

        if not test_matching_decompression(decompress, bin_data, comp_data):
            errors += 1
        if not test_matching_compression(compress, bin_data, comp_data):
            errors += 1

        print()


run_tests("yaz0", ".Yaz0", crunch64.yaz0.compress, crunch64.yaz0.decompress)
run_tests("yay0", ".Yay0", crunch64.yay0.compress, crunch64.yay0.decompress)
run_tests("mio0", ".MIO0", crunch64.mio0.compress, crunch64.mio0.decompress)

if not errors:
    print("All tests passed")
    exit(0)
else:
    print(f"{errors} tests failed")
    exit(1)
