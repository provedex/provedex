# Sidecar integration guide

`provedex-agent` is a single Rust binary that exposes a localhost HTTP signing API. Customer applications in any language POST event payloads as JSON; the agent signs locally with the customer's Ed25519 key and appends to the on-disk NDJSON ledger.

This is the default integration for any language other than Rust (decision in ADR 0004). Native bindings (`bindings/python`, `bindings/node`) are optional fast-paths for sub-millisecond signing.

## Authoritative API contract

The OpenAPI 3 spec at [`docs/spec/openapi.yaml`](../spec/openapi.yaml) is the canonical contract. It is generated from the agent source on every release via `provedex-agent --print-openapi`, so it cannot drift from the implementation. The prose and code samples below are illustrative; when in doubt, trust the OpenAPI spec.

To generate clients, run an OpenAPI generator against `docs/spec/openapi.yaml`. Example:

```
openapi-generator-cli generate -i docs/spec/openapi.yaml -g python -o ./client-python
```

## Install

After v1 release, prebuilt binaries land on GitHub Releases. Until then:

```
git clone https://github.com/provedex/provedex
cd provedex
cargo build --release -p provedex-agent
sudo install -m 755 target/release/provedex-agent /usr/local/bin/
```

## Run

```
provedex-agent
```

Defaults:

- Listen: `127.0.0.1:8765` (loopback only).
- Ledger: `~/.provedex/ledger.ndjson`.
- Key: `~/.provedex/keys/ed25519.key` (auto-generated on first run).

Override via flags or environment variables:

```
provedex-agent --listen 127.0.0.1:9001
PROVEDEX_LEDGER=/var/log/provedex/ledger.ndjson provedex-agent
PROVEDEX_KEY=/etc/provedex/key provedex-agent
```

The agent refuses to bind to a non-loopback address without `--insecure-allow-public`. There is no auth on the API; production deployments must front it with TLS plus auth (Envoy, nginx, sidecar in a service mesh).

## API

### `GET /v1/healthz`

```
curl http://127.0.0.1:8765/v1/healthz
```

Response:

```json
{
  "status": "ok",
  "session_id": "...",
  "pubkey": "..."
}
```

### `POST /v1/sign`

Body: `{ "event": <AgentEvent JSON> }`. Returns the full SignedEvent.

```
curl -X POST http://127.0.0.1:8765/v1/sign \
  -H 'content-type: application/json' \
  -d '{"event":{"type":"SessionStarted","payload":{"agent_id":"demo","model_id":"m","session_id":"s"}}}'
```

Response:

```json
{
  "seq": 0,
  "timestamp_nanos": 1778000000000000000,
  "event": {...},
  "parent_hash": "0000...",
  "self_hash": "ba3c...",
  "signature": "e26b...",
  "signer_pubkey": "15d2..."
}
```

### `POST /v1/verify`

Walks the ledger, verifies every signature and every parent_hash link.

```
curl -X POST http://127.0.0.1:8765/v1/verify
```

Response:

```json
{
  "status": "valid",
  "event_count": 7,
  "broken_at_seq": null,
  "broken_reason": null,
  "root_hash": "..."
}
```

## Per-language clients

Each is a thin HTTP wrapper. None require a published package.

### Python

```python
import json, urllib.request

def sign(event):
    req = urllib.request.Request(
        "http://127.0.0.1:8765/v1/sign",
        data=json.dumps({"event": event}).encode(),
        method="POST",
        headers={"content-type": "application/json"},
    )
    return json.loads(urllib.request.urlopen(req).read())

signed = sign({
    "type": "ModelInvoked",
    "payload": {
        "model_id": "gpt-4o",
        "prompt_sha256": "9f3b...",
        "response_sha256": "a1c2...",
        "prompt_tokens": 482,
        "response_tokens": 71,
    },
})
print(signed["seq"], signed["self_hash"])
```

### Node / TypeScript

```ts
async function sign(event: object): Promise<any> {
  const res = await fetch("http://127.0.0.1:8765/v1/sign", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ event }),
  });
  if (!res.ok) throw new Error(await res.text());
  return await res.json();
}
```

### Java

```java
import java.net.URI;
import java.net.http.*;
import java.net.http.HttpRequest.BodyPublishers;
import java.net.http.HttpResponse.BodyHandlers;

HttpRequest req = HttpRequest.newBuilder()
    .uri(URI.create("http://127.0.0.1:8765/v1/sign"))
    .header("content-type", "application/json")
    .POST(BodyPublishers.ofString(eventJson))
    .build();
HttpResponse<String> resp = httpClient.send(req, BodyHandlers.ofString());
```

### Go

```go
body, _ := json.Marshal(map[string]any{"event": event})
resp, err := http.Post(
    "http://127.0.0.1:8765/v1/sign",
    "application/json",
    bytes.NewReader(body),
)
```

### Ruby

```ruby
require "net/http"
require "json"

res = Net::HTTP.post(
  URI("http://127.0.0.1:8765/v1/sign"),
  { event: event }.to_json,
  "content-type" => "application/json",
)
```

### PHP

```php
$ch = curl_init("http://127.0.0.1:8765/v1/sign");
curl_setopt($ch, CURLOPT_POST, true);
curl_setopt($ch, CURLOPT_HTTPHEADER, ["content-type: application/json"]);
curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode(["event" => $event]));
curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
$signed = json_decode(curl_exec($ch), true);
```

## Verify your integration

After signing some events from your application:

```
provedex verify
```

A green report (`status: valid`) means every event your app emitted is cryptographically chained and verifiable by anyone with the public key.

If a verify run ever returns `status: broken`, the chain is broken; check whether something is writing to the ledger outside the sidecar.

## Out of scope for v1

- TLS support for non-loopback binds.
- Bearer-token auth on the API.
- Multi-tenant isolation (multiple key namespaces in one agent process).
- Aggregator forwarding (push to hosted aggregator).
- SIEM exporters (Splunk, Datadog, Elastic).
- Streaming sign API.

Each lands in a follow-up issue.
