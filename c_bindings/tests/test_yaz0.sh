#! /usr/bin/env bash

set -e

for i in test_data/*.Yaz0; do
    [ -f "$i" ] || break
    echo "Processing:" $i

    # Remove the extension
    BIN_PATH=$(echo $i | sed 's/.Yaz0//')

    c_bindings/tests/single_yaz0.elf $BIN_PATH $i
    echo
done
