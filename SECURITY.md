# Security Policy

Bobine Deploy follows the security policy of the Bobine project: same private channels, same report key, same
response targets. Full policy: [FantasmaGlad/Bobine, SECURITY.md](https://github.com/FantasmaGlad/Bobine/blob/main/SECURITY.md).
French version: [docs/fr/SECURITY.md](docs/fr/SECURITY.md). Machine-readable policy: <https://bobine.fit/.well-known/security.txt>.

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Do not open a public issue. Use one of these private channels:

1. **GitHub private vulnerability reporting**: [Security tab, "Report a vulnerability"](https://github.com/FantasmaGlad/bobine-deploy/security/advisories/new).
2. **E-mail**: [security@bobine.fit](mailto:security@bobine.fit), encrypted with the OpenPGP key
   `23CA D324 C507 FB0F 97E6 AECA 6E4C 020E F8BD FEB2` (`gpg --locate-keys security@bobine.fit`).

Acknowledgment within 7 days, coordinated disclosure after 90 days by default.

## Scope

In scope: this application and its packages, in particular SSH host key verification, credential handling (passwords
and private keys are wiped from memory after use and never logged), network discovery, the verification of the embedded
`install.sh` and the release signing chain. Flaws in Bobine itself are reported to the Bobine repository.

## Verifying what we publish

Each release carries `SHA256SUMS` and its detached signature `SHA256SUMS.asc`, made with the Bobine release key, also
used for the package repositories:

```bash
curl -fsSL https://packages.bobine.fit/bobine.gpg | gpg --show-keys --fingerprint
# primary key: B77D 96D7 2F84 F36A CAA3  F872 9CBF 497D 829E E15A
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum --check --ignore-missing SHA256SUMS
```

The embedded `install.sh` comes from a signed Bobine release: its digest is pinned in `bobine-release.toml` and checked
at build time.
