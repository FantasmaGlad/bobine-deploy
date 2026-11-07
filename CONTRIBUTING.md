# Contributing

French version: [docs/fr/CONTRIBUTING.md](docs/fr/CONTRIBUTING.md).

## Before you start

- Open an issue to discuss any change larger than a fix.
- Vulnerabilities never go into a public issue: see [SECURITY.md](SECURITY.md).
- The project follows the Bobine code of conduct: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Contributor License Agreement

Bobine Deploy is distributed under the AGPL-3.0-or-later and may also be offered under a separate commercial license.
To keep that possible, every contribution requires agreeing to the [Contributor License Agreement](CLA.md), by
writing in the pull request: "I have read and agree to the CLA in CLA.md." Pull requests without it are not merged.

## Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Commits

Conventional Commits, lower case, short: `type(scope): description`, for example `fix(ssh): refuse a changed host key`.
