# Bobine Deploy

Installer et maintenir [Bobine](https://github.com/FantasmaGlad/Bobine) sur les machines de votre réseau, en interface
graphique ou en ligne de commande. Bobine Deploy trouve le mini PC sur le réseau local, s'y connecte en SSH, vérifie le
matériel et la politique de privilèges, puis déroule l'installateur de Bobine avec une progression en temps réel.

Version anglaise : [README.md](README.md). Site : <https://bobine.fit>.

## État

La version 0.0.1 est l'assistant d'installation livré jusqu'ici dans le dépôt de Bobine, déplacé ici sans changement de
comportement. Il fonctionne sous Linux, Windows et macOS, en x86_64 et ARM64. Une réécriture native (sans WebView, avec
parité complète entre l'interface, la ligne de commande et une interface d'automatisation JSON-RPC) est prévue pour la
série 0.x.

## Téléchargement

Les paquets sont joints à chaque [release](https://github.com/FantasmaGlad/bobine-deploy/releases) : `.deb` et `.rpm`
pour Linux, installateur NSIS pour Windows, `.dmg` pour macOS. Vérifiez-les avant usage :

```bash
curl -fsSL https://packages.bobine.fit/bobine.gpg | gpg --import
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --check --ignore-missing SHA256SUMS
```

## Ligne de commande

Le même exécutable fonctionne sans fenêtre quand on lui donne une sous-commande :

```bash
bobine-deploy scan [--subnet 192.168.2.0/24]
bobine-deploy host-key --host bobine.local
bobine-deploy inspect --host bobine.local --user admin [--key ~/.ssh/id_ed25519 | --local-keys]
bobine-deploy install --host bobine.local --user admin --channel stable --yes
bobine-deploy version
```

`--json` écrit du JSON sur stdout (un évènement par ligne pendant `install`). Les mots de passe ne passent jamais en
argument : `BOBINE_SSH_PASSWORD` et `BOBINE_ROOT_PASSWORD`, ou `--password-stdin` et `--root-password-stdin`. La clé
d'hôte SSH est vérifiée contre `~/.ssh/known_hosts` avant tout envoi de mot de passe. Codes de sortie : 0 succès,
1 échec, 2 usage, 3 cible injoignable, 4 authentification refusée.

## Lien avec Bobine

Bobine Deploy ne réimplémente pas l'installation : il téléverse et pilote `install.sh` d'une release signée de Bobine
(épinglée dans [`bobine-release.toml`](bobine-release.toml), vérifiée par
[`scripts/sync-install-sh.py`](scripts/sync-install-sh.py) et contrôlée à la compilation) et lit sa sortie
`--progress=json`.

## Compilation

```bash
cargo test --workspace
python3 scripts/bundle.py --out dist
```

Il faut Rust (voir `rust-toolchain.toml`), Node.js pour la CLI Tauri et, sous Linux, les fichiers de développement de
WebKitGTK 4.1.

## Licence et contributions

[GNU AGPL-3.0 ou ultérieure](LICENSE). Contributions bienvenues selon [docs/fr/CONTRIBUTING.md](docs/fr/CONTRIBUTING.md).
Vulnérabilités : [docs/fr/SECURITY.md](docs/fr/SECURITY.md).
