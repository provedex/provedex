# Deployment manifests

Reference manifests for running `provedex-agent` in common environments. Copy and adapt for your infrastructure; these are starting points, not final production configurations.

## Files

- `systemd/provedex-agent.service` - Linux systemd unit. Sandboxed via `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome=true`. Reads env-file overrides from `/etc/default/provedex-agent`.
- `launchd/com.provedex.agent.plist` - macOS LaunchDaemon. Logs to `/usr/local/var/log/`.
- `k8s/sidecar-example.yaml` - Kubernetes Pod with the agent as a sidecar container alongside a customer voice agent. Demonstrates the localhost-only signing pattern: customer app POSTs to `127.0.0.1:8765/v1/sign`.

## Quick install

### Linux (systemd)

```
sudo install -m 755 ./target/release/provedex-agent /usr/local/bin/
sudo install -m 644 deploy/systemd/provedex-agent.service /etc/systemd/system/
sudo mkdir -p /var/lib/provedex
sudo systemctl daemon-reload
sudo systemctl enable --now provedex-agent
```

### macOS (launchd)

```
sudo install -m 755 ./target/release/provedex-agent /usr/local/bin/
sudo install -m 644 deploy/launchd/com.provedex.agent.plist /Library/LaunchDaemons/
sudo launchctl load /Library/LaunchDaemons/com.provedex.agent.plist
```

### Kubernetes

Adapt `k8s/sidecar-example.yaml` for your customer pod. Provision the `provedex-ledger` PVC and `provedex-signing-key` secret first; the secret is the customer's signing key (32 raw bytes).

```
kubectl create secret generic provedex-signing-key \
  --from-file=ed25519.key=./key/ed25519.key
kubectl apply -f deploy/k8s/sidecar-example.yaml
```
