"""SHA256SUMS dont la signature OpenPGP vient de la clé de publication épinglée (trousseau temporaire)."""
import re
import subprocess
import tempfile
from pathlib import Path

KEY = Path(__file__).resolve().parents[2] / "keys" / "bobine-release-signing.asc"
FINGERPRINT = "B77D96D72F84F36ACAA3F8729CBF497D829EE15A"


def verified_sums(release: Path) -> str:
    sums, signature = release / "SHA256SUMS", release / "SHA256SUMS.asc"
    with tempfile.TemporaryDirectory() as home:
        gpg = ["gpg", "--homedir", home, "--batch", "--status-fd", "1"]
        subprocess.run(gpg + ["--import", str(KEY)], check=True, capture_output=True)
        status = subprocess.run(gpg + ["--verify", str(signature), str(sums)], capture_output=True, text=True).stdout
    if not re.search(rf"^\[GNUPG:\] VALIDSIG .* {FINGERPRINT}$", status, re.M):
        raise SystemExit("SHA256SUMS : signature absente ou d'une autre clé")
    return sums.read_text(encoding="utf-8")
