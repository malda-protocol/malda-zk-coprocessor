#!/usr/bin/env bash
set -euo pipefail

# ZkCoprocessor - Integration Tests (Mainnet)
#
# Mainnet `(chain, l1_inclusion)` matrix in `malda_rs/tests/integration_tests.rs`:
# 1) LINEA, l1=false -> test_exec_get_proof_data_on_linea
# 2) LINEA, l1=true -> test_exec_get_proof_data_on_linea_with_l1_inclusion
# 3) BASE, l1=false -> test_exec_get_proof_data_on_base
# 4) BASE, l1=true -> test_exec_get_proof_data_on_base_with_l1_inclusion
# 5) OPTIMISM, l1=false -> test missing...
# 6) OPTIMISM, l1=true -> test missing...
# 7) ETHEREUM, l1=false -> test_exec_get_proof_data_on_ethereum
# 8) ETHEREUM, l1=true -> test_exec_get_proof_data_on_ethereum_with_l1_inclusion (#[should_panic])

if [[ -z "${ALCHEMY_API_KEY:-}" ]]; then
  echo "Warning: ALCHEMY_API_KEY is not set. RPC URLs must be configured manually." >&2
fi

usage() {
  cat <<USAGE
Usage:
  ./run-tests.sh <section>

Sections:
  1  Linea no L1 inclusion
  2  Linea with L1 inclusion
  3  Base no L1 inclusion
  4  Base with L1 inclusion
  5  Optimism no L1 inclusion
  6  Optimism with L1 inclusion
  7  Ethereum no L1 inclusion
  8  Ethereum with L1 inclusion (expected panic)
  all Run sections 1..8 in order
USAGE
}

run_cargo_test() {
  local test_name="$1"
  cargo test -p malda_rs --features guest \
    --test integration_tests "${test_name}" \
    -- --exact --nocapture
}

wait_for_commitment_ready() {
  local endpoint="$1"
  local container_name="$2"
  local timeout_secs="${3:-120}"
  local started_at
  started_at="$(date +%s)"

  while true; do
    local body=""
    body="$(curl -fsS --max-time 2 "${endpoint}" 2>/dev/null || true)"

    # Ready only when response body starts with JSON object.
    if [[ "${body}" =~ ^[[:space:]]*\{ ]]; then
      return 0
    fi

    local now
    now="$(date +%s)"
    if (( now - started_at >= timeout_secs )); then
      echo "Error: commitment endpoint not ready after ${timeout_secs}s: ${endpoint}" >&2
      echo "Last response body: ${body:-<empty>}" >&2
      docker logs --tail 50 "${container_name}" 2>/dev/null || true
      return 1
    fi

    sleep 2
  done
}

start_op_node_mainnet() {
  if container_is_running op-node-commitments-op-mainnet; then
    echo "Info: reusing running container op-node-commitments-op-mainnet" >&2
  elif container_exists op-node-commitments-op-mainnet; then
    echo "Error: container op-node-commitments-op-mainnet exists but is not running." >&2
    echo "Start or remove it manually, then retry." >&2
    return 1
  else
    docker run --detach --rm \
      --name op-node-commitments-op-mainnet \
      -p 9547:9547 \
      -p 9223:9223 \
      koxu1996/op-node \
      --l1=https://ethereum-rpc.publicnode.com \
      --l1.rpckind=basic \
      --l1.beacon=https://ethereum-beacon-api.publicnode.com \
      --network=op-mainnet \
      --rpc.addr=0.0.0.0 \
      --rpc.port=9547 \
      --p2p.listen.ip=0.0.0.0 \
      --p2p.listen.tcp=9223 \
      --gossip-only >/dev/null
  fi

  wait_for_commitment_ready \
    "http://127.0.0.1:9547/gossip_getSequencerCommitment" \
    "op-node-commitments-op-mainnet"
}

start_base_node_mainnet() {
  if container_is_running op-node-commitments-base-mainnet; then
    echo "Info: reusing running container op-node-commitments-base-mainnet" >&2
  elif container_exists op-node-commitments-base-mainnet; then
    echo "Error: container op-node-commitments-base-mainnet exists but is not running." >&2
    echo "Start or remove it manually, then retry." >&2
    return 1
  else
    docker run --detach --rm \
      --name op-node-commitments-base-mainnet \
      -p 9548:9547 \
      -p 9224:9223 \
      koxu1996/op-node \
      --l1=https://ethereum-rpc.publicnode.com \
      --l1.rpckind=basic \
      --l1.beacon=https://ethereum-beacon-api.publicnode.com \
      --network=base-mainnet \
      --rpc.addr=0.0.0.0 \
      --rpc.port=9547 \
      --p2p.listen.ip=0.0.0.0 \
      --p2p.listen.tcp=9223 \
      --gossip-only >/dev/null
  fi

  wait_for_commitment_ready \
    "http://127.0.0.1:9548/gossip_getSequencerCommitment" \
    "op-node-commitments-base-mainnet"
}

cleanup_container() {
  local name="$1"
  docker stop "${name}" 2>/dev/null || true
}

container_is_running() {
  local name="$1"
  docker ps --filter "name=^/${name}$" --format '{{.Names}}' | grep -qx "${name}"
}

container_exists() {
  local name="$1"
  docker ps -a --filter "name=^/${name}$" --format '{{.Names}}' | grep -qx "${name}"
}

run_section_1() {
  # 1. Testing Linea - no L1 inclusion (mainnet)
  export RPC_URL_LINEA="${RPC_URL_LINEA:-https://linea-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export BEACON_API_URL_LINEA="${BEACON_API_URL_LINEA:-https://linea-beacon-api.publicnode.com}"
  run_cargo_test tests::test_exec_get_proof_data_on_linea
}

run_section_2() {
  # 2. Testing Linea - with L1 inclusion (mainnet)
  export RPC_URL_LINEA="${RPC_URL_LINEA:-https://linea-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export BEACON_API_URL_LINEA="${BEACON_API_URL_LINEA:-https://linea-beacon-api.publicnode.com}"
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"

  start_op_node_mainnet
  run_cargo_test tests::test_exec_get_proof_data_on_linea_with_l1_inclusion
}

run_section_3() {
  # 3. Testing Base - no L1 inclusion (mainnet)
  export RPC_URL_BASE="${RPC_URL_BASE:-https://base-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_BASE="${SEQUENCER_REQUEST_BASE:-http://127.0.0.1:9548/gossip_getSequencerCommitment}"

  start_base_node_mainnet
  run_cargo_test tests::test_exec_get_proof_data_on_base
}

run_section_4() {
  # 4. Testing Base - with L1 inclusion (mainnet)
  # Note: This test is temporarily disabled.
  # Internal logic appears to be broken at the moment.
  export RPC_URL_BASE="${RPC_URL_BASE:-https://base-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"
  export RPC_URL_BASE_FALLBACK="${RPC_URL_BASE_FALLBACK:-$RPC_URL_BASE}"
  export RPC_URL_ETHEREUM_FALLBACK="${RPC_URL_ETHEREUM_FALLBACK:-$RPC_URL_ETHEREUM}"

  start_op_node_mainnet
  run_cargo_test tests::test_exec_get_proof_data_on_base_with_l1_inclusion
  cleanup_container op-node-commitments-op-mainnet
  #echo "Warning: section 4 is temporarily skipped due to known internal logic issues." >&2
  #echo "Skipped test: tests::test_exec_get_proof_data_on_base_with_l1_inclusion" >&2
}

run_section_5() {
  # 5. Testing Optimism - no L1 inclusion (mainnet)
  # Note: This test is temporarily disabled.
  # Current failure: "Failed to decode return data: Overrun".
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"

  #start_op_node_mainnet
  #run_cargo_test tests::test_exec_get_proof_data_on_optimism
  echo "Warning: section 5 is temporarily skipped due to decoding overrun." >&2
  echo "Skipped test: tests::test_exec_get_proof_data_on_optimism" >&2
}

run_section_6() {
  # 6. Testing Optimism - with L1 inclusion (mainnet)
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"
  export RPC_URL_OPTIMISM_FALLBACK="${RPC_URL_OPTIMISM_FALLBACK:-$RPC_URL_OPTIMISM}"
  export RPC_URL_ETHEREUM_FALLBACK="${RPC_URL_ETHEREUM_FALLBACK:-$RPC_URL_ETHEREUM}"

  #start_op_node_mainnet
  #run_cargo_test tests::test_exec_get_proof_data_on_optimism_with_l1_inclusion
  echo "Warning: section 6 is temporarily skipped due to Alchemy RPC issues." >&2
  echo "Skipped test: tests::test_exec_get_proof_data_on_optimism_with_l1_inclusion" >&2
}

run_section_7() {
  # 7. Testing Ethereum - no L1 inclusion (mainnet)
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"

  start_op_node_mainnet
  run_cargo_test tests::test_exec_get_proof_data_on_ethereum
}

run_section_8() {
  # 8. Testing Ethereum - with L1 inclusion (mainnet, expected panic)
  # This test is annotated with #[should_panic] and should pass by panicking.
  export RPC_URL_ETHEREUM="${RPC_URL_ETHEREUM:-https://eth-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export RPC_URL_OPTIMISM="${RPC_URL_OPTIMISM:-https://opt-mainnet.g.alchemy.com/v2/${ALCHEMY_API_KEY}}"
  export SEQUENCER_REQUEST_OPTIMISM="${SEQUENCER_REQUEST_OPTIMISM:-http://127.0.0.1:9547/gossip_getSequencerCommitment}"

  start_op_node_mainnet
  run_cargo_test tests::test_exec_get_proof_data_on_ethereum_with_l1_inclusion
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

case "$1" in
  1) run_section_1 ;;
  2) run_section_2 ;;
  3) run_section_3 ;;
  4) run_section_4 ;;
  5) run_section_5 ;;
  6) run_section_6 ;;
  7) run_section_7 ;;
  8) run_section_8 ;;
  all)
    run_section_1
    run_section_2
    run_section_3
    run_section_4
    run_section_5
    run_section_6
    run_section_7
    run_section_8
    ;;
  *)
    usage
    exit 1
    ;;
esac
