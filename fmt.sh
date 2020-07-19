#!/bin/bash
repos="asciiscope framebuffer infocus scratch"
for x in $repos; do
    pushd $x
    cargo fmt
    popd
done
