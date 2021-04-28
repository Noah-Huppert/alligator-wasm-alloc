#!/usr/bin/env bash
BENCH_DIR=./benchmarks-out
BENCH_F_PREFIX=alloc-report-

die() {
    echo "Error: $@" >&2
    exit 1
}


check() {
    if [[ "$?" != "0" ]]; then
	   die "$@"
    fi
}

while getopts "h" opt; do
    case "$opt" in
	   h)
		  cat <<EOF
benchmarks-record.sh - Run benchmarks to record relevant segments of data

USAGE

    benchmarks-record.sh [-h]

OPTIONS

    -h    Prints help text.

BEHAVIOR

    Runs benchmarks on all size classes. Saves reports to "$BENCH_DIR/${BENCH_F_PREFIX}-<size class>-<segments>.csv".

EOF
		  exit 0
		  ;;
	   '?') die "Unknown option" ;;
    esac
done

mkdir -p "$BENCH_DIR"
check "Failed to create benchmark directory"

make bench-build-host BENCH=alloc-report CARGO_BARGS=--features=metrics
check "Failed to build alloc-report benchmark program"

BENCH_PROG=./target/i686-unknown-linux-gnu/debug/bench-alloc-report

size_class=0
while (($size_class <= 11)); do
    out_f="${BENCH_DIR}/${BENCH_F_PREFIX}${size_class}.csv"
    
    $BENCH_PROG --csv-header > "$out_f"
    check "Failed to write alloc-report CSV header to \"$out_f\""

    i=1
    while (($i < 10)); do
	   $BENCH_PROG "$size_class" "$i" >> "$out_f"
	   check "Failed to run alloc-report for size_class=$size_class i=$i to \"$out_f\""
	   
	   i=$((i + 1))
    done

    echo "Finished size class $size_class benchmarks"
    
    size_class=$(($size_class + 1))
done
