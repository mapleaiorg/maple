#!/usr/bin/env bash
set -euo pipefail

# Publish chain for maple-runtime and its crates.io dependencies.
# Default mode is local dry-run for safety.

MODE="local-dry-run"
ALLOW_DIRTY=0
NO_VERIFY=0
WAIT_SECONDS=30
START_FROM=""

CRATES=(
  "rcf-types"
  "rcf-meaning"
  "rcf-intent"
  "rcf-commitment"
  "rcf-validator"
  "aas-types"
  "aas-identity"
  "aas-capability"
  "aas-policy"
  "aas-ledger"
  "aas-service"
  "resonator-types"
  "resonator-profiles"
  "resonator-commitment"
  "resonator-consequence"
  "resonator-memory"
  "resonator-conversation"
  "resonator-observability"
  "resonator-conformance"
  "maple-storage"
  "maple-runtime"
)

usage() {
  cat <<'EOF'
Usage:
  scripts/publish-maple-runtime.sh [options]

Options:
  --dry-run              Run local packaging preflight (cargo package --no-verify) (default)
  --publish-dry-run      Run cargo publish --dry-run (requires upstream deps already on crates.io)
  --execute              Run real cargo publish
  --allow-dirty          Pass --allow-dirty to package/publish commands
  --no-verify            Pass --no-verify to publish/execute mode (dry-run mode already no-verify)
  --wait-seconds N       Seconds to wait between publishes in execute mode (default: 30)
  --from CRATE           Start from CRATE in publish chain (for resuming)
  --help                 Show help

Examples:
  scripts/publish-maple-runtime.sh --dry-run
  scripts/publish-maple-runtime.sh --publish-dry-run --from maple-runtime
  scripts/publish-maple-runtime.sh --execute
  scripts/publish-maple-runtime.sh --execute --from aas-policy --wait-seconds 45
EOF
}

log() {
  printf '[publish-maple-runtime] %s\n' "$*"
}

die() {
  printf '[publish-maple-runtime] ERROR: %s\n' "$*" >&2
  exit 1
}

is_member() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    if [[ "$item" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)
        MODE="local-dry-run"
        shift
        ;;
      --publish-dry-run)
        MODE="publish-dry-run"
        shift
        ;;
      --execute)
        MODE="execute"
        shift
        ;;
      --allow-dirty)
        ALLOW_DIRTY=1
        shift
        ;;
      --no-verify)
        NO_VERIFY=1
        shift
        ;;
      --wait-seconds)
        [[ $# -ge 2 ]] || die "--wait-seconds requires a numeric value"
        WAIT_SECONDS="$2"
        shift 2
        ;;
      --from)
        [[ $# -ge 2 ]] || die "--from requires a crate name"
        START_FROM="$2"
        shift 2
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1 (use --help)"
        ;;
    esac
  done
}

check_prereqs() {
  command -v cargo >/dev/null 2>&1 || die "cargo not found in PATH"
  command -v git >/dev/null 2>&1 || die "git not found in PATH"
  [[ -f Cargo.toml ]] || die "run from repo root (Cargo.toml missing)"
  grep -q '^\[workspace\]' Cargo.toml || die "Cargo.toml is not workspace root"

  if [[ "$ALLOW_DIRTY" -ne 1 ]]; then
    if [[ -n "$(git status --porcelain)" ]]; then
      die "working tree is dirty; commit/stash or pass --allow-dirty"
    fi
  fi

  if [[ -n "$START_FROM" ]]; then
    is_member "$START_FROM" "${CRATES[@]}" || die "--from crate '$START_FROM' not in publish chain"
  fi
}

publish_one() {
  local crate="$1"
  local -a args

  case "$MODE" in
    local-dry-run)
      args=(package -p "$crate" --no-verify)
      ;;
    publish-dry-run)
      args=(publish -p "$crate" --dry-run)
      ;;
    execute)
      args=(publish -p "$crate")
      ;;
    *)
      die "unsupported mode: $MODE"
      ;;
  esac

  if [[ "$ALLOW_DIRTY" -eq 1 ]]; then
    args+=(--allow-dirty)
  fi
  if [[ "$NO_VERIFY" -eq 1 && "$MODE" != "local-dry-run" ]]; then
    args+=(--no-verify)
  fi

  log "cargo ${args[*]}"
  cargo "${args[@]}"
}

main() {
  parse_args "$@"
  check_prereqs

  local start_index=0
  if [[ -n "$START_FROM" ]]; then
    local i
    for i in "${!CRATES[@]}"; do
      if [[ "${CRATES[$i]}" == "$START_FROM" ]]; then
        start_index="$i"
        break
      fi
    done
  fi

  log "mode: $MODE"
  log "crates: ${CRATES[*]}"
  if [[ "$MODE" == "execute" ]]; then
    log "waiting ${WAIT_SECONDS}s between publishes for index propagation"
  elif [[ "$MODE" == "publish-dry-run" ]]; then
    log "publish dry-run mode requires already-published upstream crates"
  else
    log "local dry-run mode packages crates without network dependency verification"
  fi

  local idx
  for (( idx=start_index; idx<${#CRATES[@]}; idx++ )); do
    local crate="${CRATES[$idx]}"
    publish_one "$crate"
    if [[ "$MODE" == "execute" && "$idx" -lt $((${#CRATES[@]} - 1)) ]]; then
      log "sleep ${WAIT_SECONDS}s"
      sleep "$WAIT_SECONDS"
    fi
  done

  log "completed successfully"
}

main "$@"
