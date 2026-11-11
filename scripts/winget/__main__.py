"""Usage : python3 scripts/winget <tag vX.Y.Z> <dossier : SHA256SUMS et SHA256SUMS.asc> <dossier de sortie>

Vérifie la signature de SHA256SUMS avec la clé épinglée, puis écrit les manifestes winget. Rien n'est soumis : la PR vers
microsoft/winget-pkgs reste une action du mainteneur.
"""
import sys
from pathlib import Path

from manifests import manifests
from signature import verified_sums


def main() -> int:
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    tag, release, out = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    for name, content in manifests(verified_sums(release), tag).items():
        (out / name).write_text(content, encoding="utf-8")
        print(out / name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
