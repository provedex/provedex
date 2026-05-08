#!/usr/bin/env bash
# Sidecar HTTP latency benchmark. Each scenario gets its own fresh agent +
# fresh ledger so verify scenarios are not polluted by sign-load runs.
#
# Usage: bash benchmarks/agent-http/run.sh
# Requires: cargo, oha (cargo install oha), python3.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="benchmarks/agent-http/out"
mkdir -p "$OUT_DIR"
rm -f "$OUT_DIR"/*.json

PORT=8800
URL="http://127.0.0.1:$PORT"

AGENT_PID=""
SANDBOX=""
trap 'cleanup' EXIT INT TERM

cleanup() {
  stop_agent
  [ -n "$SANDBOX" ] && rm -rf "$SANDBOX"
}

stop_agent() {
  if [ -n "$AGENT_PID" ] && kill -0 "$AGENT_PID" 2>/dev/null; then
    kill "$AGENT_PID" 2>/dev/null || true
    wait "$AGENT_PID" 2>/dev/null || true
  fi
  AGENT_PID=""
}

start_agent() {
  stop_agent
  [ -n "$SANDBOX" ] && rm -rf "$SANDBOX"
  SANDBOX="$(mktemp -d -t provedex-bench-XXXXXX)"
  PROVEDEX_LEDGER="$SANDBOX/ledger.ndjson" \
  PROVEDEX_KEY="$SANDBOX/ed25519.key" \
  PROVEDEX_AGENT_LISTEN="127.0.0.1:$PORT" \
  RUST_LOG=warn \
  ./target/release/provedex-agent --rate-limit-off &> "$SANDBOX/agent.log" &
  AGENT_PID=$!
  for _ in $(seq 1 50); do
    if curl -sSf "$URL/v1/healthz" > /dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  echo "agent failed to start; tail log:"
  tail -20 "$SANDBOX/agent.log"
  exit 1
}

run_oha() {
  local label="$1"; shift
  local fname
  fname=$(echo "$label" | tr -c '[:alnum:]._-' '_')
  local out="$OUT_DIR/$fname.json"
  oha --output-format json --no-tui "$@" > "$out"
  LABEL="$label" OUT="$out" python3 "$REPO_ROOT/benchmarks/agent-http/format.py"
}

echo "=== building provedex-agent (release) ==="
cargo build --release --quiet -p provedex-agent

# Pre-built request body for /v1/sign.
BODY_TMP=$(mktemp -t provedex-body-XXXXXX)
cat > "$BODY_TMP" <<'JSON'
{"event":{"type":"ModelInvoked","payload":{"model_id":"gpt-4o","prompt_sha256":"9f3b2a1c0d4e5f6789abcdef0123456789abcdef0123456789abcdef01234567","response_sha256":"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef","prompt_tokens":482,"response_tokens":71}}}
JSON
trap 'cleanup; rm -f "$BODY_TMP"' EXIT INT TERM

# --- /v1/healthz baseline (fresh agent, empty ledger).
echo
echo "=== /v1/healthz baseline ==="
start_agent
run_oha "healthz @ c=50, n=5000" \
  -c 50 -n 5000 -m GET "$URL/v1/healthz"

# --- /v1/sign across concurrencies (one fresh agent per concurrency).
echo
echo "=== /v1/sign across concurrency ==="
for C in 1 10 100; do
  start_agent
  run_oha "sign @ c=$C, n=5000" \
    -c $C -n 5000 -m POST -T application/json -D "$BODY_TMP" "$URL/v1/sign"
done

# --- /v1/verify at varying chain sizes. Fresh agent + fresh ledger per
# scenario so each verify call sees exactly $SIZE events.
echo
echo "=== /v1/verify at varying chain sizes ==="
for SIZE in 100 1000 10000; do
  start_agent
  echo "  populating chain to $SIZE events..."
  oha --no-tui -c 50 -n "$SIZE" -m POST -T application/json -D "$BODY_TMP" "$URL/v1/sign" > /dev/null
  run_oha "verify @ chain=$SIZE, c=1, n=200" \
    -c 1 -n 200 -m POST "$URL/v1/verify"
done

echo
echo "=== done ==="
