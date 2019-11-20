#!/usr/bin/env python3

import os

output_counter = 0

def perform_capture(cx, cy, cz, ch, cp, sa):
    global output_counter
    os.system('cargo run ' + ' '.join([str(e) for e in [cx, cy, cz, ch, cp, sa]]))
    os.system('rm -r denoiser/training/' + str(output_counter))
    os.system('mkdir denoiser/training/' + str(output_counter))
    os.system('mv denoiser/training/*.dat denoiser/training/' + str(output_counter))
    output_counter += 1

positions = [
    [100, 100, 60],
    [100, 200, 60],
    [200, 200, 60],
    [200, 100, 60],
    [200, 200, 160],
]

angles = [
    [-3, -0.1],
    [-2, -0.1],
    [-1, -0.1],
    [0, -0.1],
    [1, -0.1],
    [2, -0.1],
    [3, -0.1],
]

suns = [
    -1.2,
    -0.7,
    0.0,
    0.7,
    1.2,
]

for position in positions:
    for angle in angles:
        for sun in suns:
            args = position + angle + [sun]
            perform_capture(*args)