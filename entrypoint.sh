#!/bin/sh
set -e

ELF="/app/malda_rs/bin/get-proof-data.bin"
IDFILE="/app/malda_rs/src/elfs_ids.rs"

printf "================ RISC0 Guest ELF ================\n"
printf "Path:      %s\n" "$ELF"
printf "SHA256:    %s\n" $(sha256sum "$ELF" | cut -d' ' -f1)
printf "Image ID:  "
grep "_ID" "$IDFILE" | awk -F'= \\[|];' '{print "[" $2 "]"}'
printf "=================================================\n\n"

# Execute provided command.
exec "$@"
