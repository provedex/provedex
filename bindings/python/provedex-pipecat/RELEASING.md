# Release process for provedex-pipecat

Pre-release checklist:

1. All tests pass locally and in CI.
2. `pyproject.toml` version bumped if shipping a new version.
3. Tag the binding release: `git tag pipecat-vX.Y.Z` (use a binding-scoped tag prefix so it does not collide with the agent's `vX.Y.Z` tags).

Publish to PyPI:

```
cd bindings/python/provedex-pipecat
python -m pip install --upgrade build twine
python -m build
python -m twine check dist/*
python -m twine upload dist/*
```

After publish:

1. Verify `pip install provedex-pipecat` from a clean venv pulls the new version.
2. Confirm the README on PyPI renders correctly (long_description comes from `README.md`).
3. Bump the `provedex-pipecat` row in the root `README.md` Components table if anything material changed.

Yank policy:

```
python -m twine yank provedex-pipecat==X.Y.Z
```

A yank does not delete the version; it stops new dependents from picking it up. Existing lockfiles keep their pin. Use yank when a published version has a hard bug; publish a fixed `X.Y.Z+1` and document the yank reason in the next release notes.

Out of scope here:

- The Rust agent + CLI publish process lives in the root `RELEASING.md`.
- PyPI account ownership and 2FA recovery live in 1Password under the `provedex-pipecat-pypi` entry. Not in this repo.
