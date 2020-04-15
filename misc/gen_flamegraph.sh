#!/bin/sh

perf script | stackcollapse-perf.pl | flamegraph.pl > graph.svg
rm perf.data
firefox graph.svg
