# Release process for provedex-langchain

Pre-release checklist:

1. All tests pass locally and in CI.
2. `pyproject.toml` version bumped if shipping a new version.
3. Tag the binding release: `git tag langchain-vX.Y.Z` (binding-scoped prefix so it does not collide with the agent's `vX.Y.Z` tags).

Publish to PyPI:

```
cd bindings/python/provedex-langchain
python -m pip install --upgrade build twine
python -m build
python -m twine check dist/*
python -m twine upload dist/*
```

After publish:

1. Verify `pip install provedex-langchain` from a clean venv pulls the new version.
2. Confirm the README on PyPI renders correctly (long_description from `README.md`).
3. Update the `provedex-langchain` row in the root `README.md` Components table if anything material changed.

Yank policy: same as `provedex-pipecat`; see `bindings/python/provedex-pipecat/RELEASING.md` for the procedure.

Out of scope here: the Rust agent + CLI publish process lives in the root `RELEASING.md`.
