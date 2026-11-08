#!/usr/bin/env python3
"""Récupère install.sh d'une release de Bobine, le vérifie et l'embarque dans vendor/install.sh.

Usage : sync-install-sh.py [--tag V5.1.1] [--check]

- sans --check : télécharge install.sh, SHA256SUMS et SHA256SUMS.asc de la release, vérifie la signature avec la clé de
  keys/bobine-release-signing.asc (dans un trousseau temporaire), vérifie l'empreinte, puis écrit vendor/install.sh et
  bobine-release.toml ;
- avec --check : refait la même vérification pour la release épinglée et compare avec le fichier embarqué, sans rien
  écrire (utilisé par la CI de publication).
"""
from __future__ import annotations

import argparse
import hashlib
import re
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PIN = ROOT / "bobine-release.toml"
VENDORED = ROOT / "vendor" / "install.sh"
KEY = ROOT / "keys" / "bobine-release-signing.asc"
FINGERPRINT = "B77D96D72F84F36ACAA3F8729CBF497D829EE15A"
BASE = "https://github.com/FantasmaGlad/Bobine/releases/download/{tag}/{name}"


def fetch(tag: str, name: str) -> bytes:
    """GitHub répond parfois 5xx sur les assets de release : trois essais espacés avant d'abandonner."""
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(BASE.format(tag=tag, name=name), timeout=60) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError) as error:
            if attempt == 3 or (isinstance(error, urllib.error.HTTPError) and error.code < 500):
                raise
            time.sleep(10 * attempt)
    raise AssertionError("inatteignable")


def verified_install_sh(tag: str) -> bytes:
    script, sums, signature = (fetch(tag, n) for n in ("install.sh", "SHA256SUMS", "SHA256SUMS.asc"))
    with tempfile.TemporaryDirectory() as home:
        work = Path(home)
        (work / "SHA256SUMS").write_bytes(sums)
        (work / "SHA256SUMS.asc").write_bytes(signature)
        gpg = ["gpg", "--homedir", home, "--batch", "--status-fd", "1"]
        subprocess.run(gpg + ["--import", str(KEY)], check=True, capture_output=True)
        status = subprocess.run(gpg + ["--verify", str(work / "SHA256SUMS.asc"), str(work / "SHA256SUMS")],
                                capture_output=True, text=True).stdout
    # VALIDSIG : empreinte de la clé signataire (sous-clé possible) ... empreinte de la clé principale en dernier.
    if not re.search(rf"^\[GNUPG:\] VALIDSIG .* {FINGERPRINT}$", status, re.M):
        raise SystemExit(f"{tag} : signature de SHA256SUMS invalide ou d'une autre clé")
    expected = next((line.split()[0] for line in sums.decode().splitlines() if line.endswith("  install.sh")), None)
    if expected != hashlib.sha256(script).hexdigest():
        raise SystemExit(f"{tag} : install.sh ne correspond pas à SHA256SUMS")
    return script


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag", help="tag de la release de Bobine (défaut : celui de bobine-release.toml)")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    pin = tomllib.loads(PIN.read_text(encoding="utf-8"))
    tag = args.tag or pin["tag"]
    script = verified_install_sh(tag)
    digest = hashlib.sha256(script).hexdigest()
    if args.check:
        if VENDORED.read_bytes() != script or pin["install_sh_sha256"] != digest:
            raise SystemExit(f"vendor/install.sh ou bobine-release.toml ne correspondent pas à la release {tag}")
        print(f"install.sh conforme à la release signée {tag} ({digest}).")
        return 0
    VENDORED.parent.mkdir(exist_ok=True)
    VENDORED.write_bytes(script)
    PIN.write_text(PIN.read_text(encoding="utf-8").splitlines()[0] + f'\ntag = "{tag}"\ninstall_sh_sha256 = "{digest}"\n',
                   encoding="utf-8")
    print(f"vendor/install.sh mis à jour depuis {tag} ({digest}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
