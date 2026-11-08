#!/usr/bin/env bash
# Vérifie les assets d'une release avant publication : exactement les paquets attendus pour chaque système et chaque
# architecture, plus SHA256SUMS et SHA256SUMS.asc ; chaque paquet listé avec sa vraie empreinte.
#
# Usage : verify-release-assets.sh <dossier> <version>
set -euo pipefail
export LC_ALL=C
DIR="${1:?dossier des assets}"
V="${2:?version}"

expected=(
    "bobine-deploy_${V}_amd64.deb"
    "bobine-deploy_${V}_arm64.deb"
    "bobine-deploy-${V}.x86_64.rpm"
    "bobine-deploy-${V}.aarch64.rpm"
    "Bobine-Deploy-Setup-${V}-x64.exe"
    "Bobine-Deploy-Setup-${V}-arm64.exe"
    "Bobine-Deploy-${V}-x64.dmg"
    "Bobine-Deploy-${V}-arm64.dmg"
    "SHA256SUMS"
    "SHA256SUMS.asc"
)

actual=$(cd "$DIR" && find . -maxdepth 1 -type f -printf '%f\n' | sort)
wanted=$(printf '%s\n' "${expected[@]}" | sort)
if [ "$actual" != "$wanted" ]; then
    echo "Assets inattendus :" >&2
    diff <(echo "$wanted") <(echo "$actual") >&2 || true
    exit 1
fi
(cd "$DIR" && sha256sum --check --strict SHA256SUMS)
listed=$(awk '{print $2}' "$DIR/SHA256SUMS" | sort)
[ "$listed" = "$(printf '%s\n' "${expected[@]}" | grep -v '^SHA256SUMS' | sort)" ] || { echo 'SHA256SUMS incomplet.' >&2; exit 1; }
echo "Release $V : ${#expected[@]} fichiers conformes."
