#!/bin/sh
set -e

# Find the generated methods.rs which contains the ELF path and image ID.
METHODS_RS=$(find /app/target -name "methods.rs" -path "*/methods-*/out/*" | head -1)
if [ -z "$METHODS_RS" ]; then
    echo "Error: Could not find methods.rs in /app/target" >&2
    exit 1
fi

# Extract ELF path from methods.rs (GET_PROOF_DATA_PATH constant).
ELF=$(grep 'GET_PROOF_DATA_PATH.*=.*"' "$METHODS_RS" | sed 's/.*"\(.*\)".*/\1/')
if [ -z "$ELF" ] || [ ! -f "$ELF" ]; then
    echo "Error: Could not find ELF file. Path from methods.rs: ${ELF:-<empty>}" >&2
    exit 1
fi

printf "================ RISC0 Guest ELF ================\n"
printf "Path:      %s\n" "$ELF"
printf "SHA256:    %s\n" $(sha256sum "$ELF" | cut -d' ' -f1)
printf "Image ID:  "
grep "GET_PROOF_DATA_ID" "$METHODS_RS" | awk -F'= \\[|];' '{print "[" $2 "]"}'
printf "=================================================\n\n"

# Execute provided command.
exec "$@"
