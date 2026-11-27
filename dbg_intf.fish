#!/usr/bin/env fish
dot -Tsvg $argv[1] >debug/graph.svg && gwenview debug/graph.svg
