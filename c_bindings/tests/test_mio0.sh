#! /usr/bin/env bash

set -e

for i in test_data/*.MIO0; do
    [ -f "$i" ] || break
    echo "Processing:" $i

    # Remove the extension
    BIN_PATH=$(echo $i | sed 's/.MIO0//')

    c_bindings/tests/test_mio0.elf $BIN_PATH $i
    echo
done
