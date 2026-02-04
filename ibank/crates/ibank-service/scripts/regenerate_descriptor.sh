#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROTO_DIR="${CRATE_DIR}/proto"
OUT_FILE="${CRATE_DIR}/src/generated/ibank_descriptor.bin"

protoc \
  --proto_path="${PROTO_DIR}" \
  --include_imports \
  --descriptor_set_out="${OUT_FILE}" \
  ibank/v1/ibank.proto

echo "Wrote ${OUT_FILE}"
