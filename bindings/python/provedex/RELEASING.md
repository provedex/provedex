# Releasing `provedex` (native Python binding)

Wheels are built by maturin and published to PyPI as `provedex`. Version tracks `provedex-core` semver.

## One-time

```bash
pip install --upgrade maturin twine
```

A PyPI API token with upload scope for the `provedex` project.

## Build the wheels + sdist

The release is normally cut by the `release-python.yml` GitHub workflow on a `python-v*` tag (manylinux x86_64, linux aarch64, macOS arm64, all abi3-py311, plus the sdist). To build locally for a smoke check:

```bash
cd bindings/python/provedex
maturin build --release
ls target/wheels/
```

## Publish

Download the workflow's `dist/` artifact (or use the local `target/wheels/`), then:

```bash
twine check dist/*
twine upload dist/*
```

## Verify

```bash
python -m venv /tmp/verify-provedex
/tmp/verify-provedex/bin/pip install provedex
/tmp/verify-provedex/bin/python -c "import provedex; print(provedex.__version__)"
```

## Tag

```bash
git tag python-v0.1.0
git push origin python-v0.1.0
```
