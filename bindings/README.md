# Language bindings

FFI wrappers around `provedex-core` so customers can emit signed events from their app's native runtime without spawning a Rust process.

## Layout

- `python/` - PyO3 wrapper. Publishes `provedex` on PyPI. Customers `pip install provedex` and call `provedex.sign(event)`.
- `node/` - napi-rs wrapper. Publishes `@provedex/core` on npm. Customers `npm install @provedex/core` and call `signEvent(payload)`.

Future bindings (Java/JNI, Go/cgo, Ruby) live as sibling directories when needed.

## Byte-compat requirement

Every binding must produce signed events that verify identically to the Rust reference. The `tests/compat/` suite (when added) hashes a known event with each binding and asserts byte equality. New bindings must pass this suite before publish.
