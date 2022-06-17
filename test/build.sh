#!/bin/bash
set -eu

SCRIPT_DIR=$(dirname -- "$( readlink -f -- "$0"; )");

test_dir="${SCRIPT_DIR}/test"

multiplier=12

for i in $(seq 1 ${multiplier}); do
    for j in $(seq 1 ${multiplier}); do
        mkdir -p "${test_dir}/${i}/${j}"
        for k in $(seq 1 ${multiplier}); do
            touch "${test_dir}/${i}/${j}/f${k}"
        done
    done
    for j in $(seq 1 ${multiplier}); do
        touch "${test_dir}/${i}/f${j}"
    done
done
