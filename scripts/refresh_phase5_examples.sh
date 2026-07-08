#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIL=(cargo run -q -p bil-cli --)
ISSUED_AT="2026-07-05T00:00:00Z"
PRIVATE_KEY="$ROOT/examples/keys/phase5-ed25519-private.der"
PUBLIC_KEY="$ROOT/examples/keys/phase5-ed25519-public.der"

if [[ ! -f "$PRIVATE_KEY" || ! -f "$PUBLIC_KEY" ]]; then
  echo "Phase 5 fixture keys are missing under examples/keys" >&2
  exit 1
fi

rm -rf \
  "$ROOT/examples/axle-proof-artifact/axle-proof-artifact.bil" \
  "$ROOT/examples/lean-proof-bundle/lean-proof-bundle.bil" \
  "$ROOT/examples/lean-proof-bundle/lean-proof-bundle.receipt.json" \
  "$ROOT/examples/ai-decision-bundle/ai-decision-bundle.bil"

"${BIL[@]}" bundle create \
  --axle "$ROOT/examples/axle-proof-artifact/verify-proof-response.json" \
  --axle-kind verify-proof \
  --out "$ROOT/examples/axle-proof-artifact/axle-proof-artifact.bil" >/dev/null
"${BIL[@]}" receipt issue \
  "$ROOT/examples/axle-proof-artifact/axle-proof-artifact.bil" \
  --mode embedded \
  --algorithm ed25519 \
  --private-key "$PRIVATE_KEY" \
  --issued-at "$ISSUED_AT" >/dev/null
cp "$PUBLIC_KEY" "$ROOT/examples/axle-proof-artifact/trust-key.der"

"${BIL[@]}" bundle create \
  --axle "$ROOT/examples/lean-proof-bundle/check-response.json" \
  --axle-kind check \
  --out "$ROOT/examples/lean-proof-bundle/lean-proof-bundle.bil" >/dev/null
"${BIL[@]}" receipt issue \
  "$ROOT/examples/lean-proof-bundle/lean-proof-bundle.bil" \
  --mode detached \
  --algorithm ed25519 \
  --private-key "$PRIVATE_KEY" \
  --issued-at "$ISSUED_AT" \
  --out "$ROOT/examples/lean-proof-bundle/lean-proof-bundle.receipt.json" >/dev/null
cp "$PUBLIC_KEY" "$ROOT/examples/lean-proof-bundle/trust-key.der"

"${BIL[@]}" bundle create \
  --axle "$ROOT/examples/ai-decision-bundle/check-response.json" \
  --axle-kind check \
  --out "$ROOT/examples/ai-decision-bundle/ai-decision-bundle.bil" >/dev/null
"${BIL[@]}" bundle institutionalize \
  "$ROOT/examples/ai-decision-bundle/ai-decision-bundle.bil" \
  --institutional "$ROOT/examples/ai-decision-bundle/institutional.json" \
  --risk "$ROOT/examples/ai-decision-bundle/risk.json" \
  --controls "$ROOT/examples/ai-decision-bundle/controls.json" >/dev/null
"${BIL[@]}" receipt issue \
  "$ROOT/examples/ai-decision-bundle/ai-decision-bundle.bil" \
  --mode embedded \
  --algorithm ed25519 \
  --private-key "$PRIVATE_KEY" \
  --issued-at "$ISSUED_AT" >/dev/null
cp "$PUBLIC_KEY" "$ROOT/examples/ai-decision-bundle/trust-key.der"

(
  cd "$ROOT"
  "${BIL[@]}" report \
    ./examples/ai-decision-bundle/ai-decision-bundle.bil \
    --kind audit \
    --format markdown \
    --trust-key ./examples/ai-decision-bundle/trust-key.der \
    >./examples/ai-decision-bundle/reports/audit-review-example.md

  "${BIL[@]}" report \
    ./examples/ai-decision-bundle/ai-decision-bundle.bil \
    --kind regulatory \
    --format markdown \
    --trust-key ./examples/ai-decision-bundle/trust-key.der \
    >./examples/ai-decision-bundle/reports/regulatory-review-example.md
)

echo "Phase 5 example bundles, receipts, and report examples refreshed."
