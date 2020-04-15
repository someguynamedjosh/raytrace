#!/bin/sh

rm graph.svg perf.data
perf record -g target/debug/raytrace
