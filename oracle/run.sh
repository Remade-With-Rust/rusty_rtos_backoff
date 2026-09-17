#!/bin/sh
# Generate the C arm of K7's backoffAlgorithm differential.
#
# `backoff_algorithm.c` is compiled VERBATIM out of the pinned checkout in the
# umbrella; this script never copies or edits it. Fetch it first with
#
#     cargo run --manifest-path tools/kairos/Cargo.toml -- oracle fetch --lib backoffAlgorithm
#
# The trace it writes is checked in, so the Rust side diffs it in CI with no C
# toolchain -- the same arrangement the kernel corpus and the heap
# differentials use.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
lib="$here/../../oracle/backoffAlgorithm"
src="$lib/source/backoff_algorithm.c"

[ -f "$src" ] || { echo "no backoff_algorithm.c at $src -- run \`kairos oracle fetch --lib backoffAlgorithm\` first" >&2; exit 1; }

cc -O2 -g -Wall -Wextra \
   -I "$lib/source/include" \
   -o "$here/backoff_driver" \
   "$src" "$here/backoff_driver.c"

"$here/backoff_driver" > "$here/backoff.trace"
echo "wrote $(wc -l < "$here/backoff.trace") lines to $here/backoff.trace"
