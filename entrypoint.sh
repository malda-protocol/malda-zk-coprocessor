#!/bin/sh
set -e

ELF="/app/target/riscv-guest/methods/guests/riscv32im-risc0-zkvm-elf/release/get-proof-data.bin"
METHODS_RS=$(find /app/target -name "methods.rs" -path "*/methods-*/out/*" | head -1)

printf "================ RISC0 Guest ELF ================\n"
printf "Path:      %s\n" "$ELF"
printf "SHA256:    %s\n" $(sha256sum "$ELF" | cut -d' ' -f1)
printf "Image ID:  "
grep "_ID" "$METHODS_RS" | awk -F'= \\[|];' '{print "[" $2 "]"}'
printf "=================================================\n\n"

# Execute provided command.
exec "$@"
