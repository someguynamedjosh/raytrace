#!/bin/sh

cd ..

cargo run 100 100 60 1 -0.1 1.2
rm -r denoiser/training/a
mkdir denoiser/training/a
mv denoiser/training/*.dat denoiser/training/a/