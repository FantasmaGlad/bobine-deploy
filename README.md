# Bobine Deploy

Install and maintain [Bobine](https://github.com/FantasmaGlad/Bobine) on the machines of your network, from a
graphical interface or from the command line. Bobine Deploy finds the mini PC on the local network, connects to it over
SSH, checks the hardware and the privilege policy, then runs the Bobine installer with live progress.

French version: [README.fr.md](README.fr.md). Website: <https://bobine.fit>.

## Status

Version 0.0.1 is the installation assistant formerly shipped inside the Bobine repository, moved here unchanged in
behavior. It runs on Linux, Windows and macOS, x86_64 and ARM64. A native rewrite (no WebView, full parity between the
interface, the command line and a JSON-RPC automation interface) is planned for the 0.x series.

## Download

Packages are attached to each [release](https://github.com/FantasmaGlad/bobine-deploy/releases): `.deb` and `.rpm`
for Linux, an NSIS installer for Windows, a `.dmg` for macOS. Check them before use:

```bash
curl -fsSL https://packages.bobine.fit/bobine.gpg | gpg --import
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
```

## Command line

The same executable runs without a window when given a subcommand:

```bash
bobine-deploy scan [--subnet 192.168.2.0/24]
bobine-deploy host-key --host bobine.local
bobine-deploy inspect --host bobine.local --user admin [--key ~/.ssh/id_ed25519 | --local-keys]
bobine-deploy install --host bobine.local --user admin --channel stable --yes
bobine-deploy version
```

`--json` prints JSON on stdout (one event per line during `install`). Passwords are never passed as arguments: use
`BOBINE_SSH_PASSWORD` and `BOBINE_ROOT_PASSWORD`, or `--password-stdin` and `--root-password-stdin`. The SSH host key is
checked against `~/.ssh/known_hosts` before any password is sent. Exit codes: 0 success, 1 failure, 2 usage,
3 unreachable target, 4 authentication refused.

## How it relates to Bobine

Bobine Deploy does not reimplement the installation: it uploads and drives `install.sh` from a signed Bobine release
(pinned in [`bobine-release.toml`](bobine-release.toml), verified by [`scripts/sync-install-sh.py`](scripts/sync-install-sh.py)
and checked at build time) and reads its `--progress=json` output.

## Build

```bash
cargo test --workspace
python3 scripts/bundle.py --out dist
```

Requires Rust (see `rust-toolchain.toml`), Node.js for the Tauri CLI, and on Linux the WebKitGTK 4.1 development files.

## License and contributions

[GNU AGPL-3.0-or-later](LICENSE). Contributions are welcome under the terms of [CONTRIBUTING.md](CONTRIBUTING.md).
Security issues: [SECURITY.md](SECURITY.md).
