#!/usr/bin/env fish
dot -Tpng $argv[1] >debug/graph.png && gwenview debug/graph.png
