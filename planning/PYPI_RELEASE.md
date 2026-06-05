# PyPI Release Guide

This document explains how to publish `maggraph` to PyPI. The workflow uses
[PyPI Trusted Publishing](https://docs.pypi.org/trusted-publishers/) (OIDC) —
**no API token or secret is needed** in GitHub after initial setup.

---

## One-time PyPI setup (do this once before the first release)

### 1. Create the PyPI project

1. Go to [pypi.org](https://pypi.org) → **Log in** → **Your projects** → **Publish**
2. Select **Add a new pending publisher**
3. Fill in:
   - **PyPI project name**: `maggraph`
   - **GitHub owner**: `AlexMercedCoder`
   - **Repository**: `MagGraph`
   - **Workflow filename**: `publish.yml`
   - **Environment name**: `pypi`

### 2. Create the GitHub environment

1. In the GitHub repo → **Settings** → **Environments** → **New environment**
2. Name it exactly `pypi`
3. Optionally add **Required reviewers** or a **deployment branch rule** (`v*`)

That's it. No secrets, no tokens. The OIDC handshake between GitHub Actions and
PyPI is automatic.

---

## Publishing a release

```bash
# 1. Bump the version in both places:
#    - python/pyproject.toml  →  version = "0.1.1"
#    - Cargo.toml (workspace) →  version = "0.1.1"

# 2. Commit and tag
git add python/pyproject.toml Cargo.toml
git commit -m "chore: release v0.1.1"
git tag v0.1.1
git push origin main --tags
```

Pushing the tag triggers `.github/workflows/publish.yml`, which:
1. Builds manylinux x86\_64 + aarch64, macOS Intel + Apple Silicon, and Windows wheels
2. Builds an sdist (source distribution as fallback)
3. Smoke-tests each wheel in a clean venv
4. Publishes all artifacts to PyPI via Trusted Publishing

The package will appear at **https://pypi.org/project/maggraph/** within a few minutes.

---

## Verifying the release

```bash
# Install from PyPI in a fresh venv
python -m venv /tmp/test-maggraph
/tmp/test-maggraph/bin/pip install maggraph==0.1.1
/tmp/test-maggraph/bin/python -c "
import maggraph
print(maggraph.__version__)
index = maggraph.open_index('examples/basic/knowledge_graph')
print(index.list_nodes())
"
```

---

## Dry-run without publishing

The publish workflow also runs on PRs that touch `python/**` — it builds all
wheels and runs smoke tests but skips the publish step. This lets you verify
the build before tagging.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `403 Forbidden` from PyPI | Re-check the pending publisher settings — owner, repo, workflow filename, and environment name must match exactly |
| aarch64 wheel missing | The cross-compile uses QEMU in the `maturin-action` container; check the `aarch64` matrix job logs |
| Wheel not found in `--find-links` smoke | Confirm `maturin build --release` ran before the smoke step |
| `maturin` version conflict | Pin `maturin>=1.7,<2` in `pyproject.toml` build-system requirements |
