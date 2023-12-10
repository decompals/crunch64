#! /usr/bin/env bash

set -e

for i in test_data/*.Yay0; do
    [ -f "$i" ] || break
    echo "Processing:" $i

    # Remove the extension
    BIN_PATH=$(echo $i | sed 's/.Yay0//')

    c_bindings/tests/test_yay0.elf $BIN_PATH $i
    echo
done
