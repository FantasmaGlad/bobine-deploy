# Politique de sécurité

Bobine Deploy suit la politique de sécurité du projet Bobine : mêmes canaux privés, même clé de signalement, mêmes
délais. Politique complète : [FantasmaGlad/Bobine, docs/fr/SECURITY.md](https://github.com/FantasmaGlad/Bobine/blob/main/docs/fr/SECURITY.md).
Version anglaise : [SECURITY.md](../../SECURITY.md). Politique lisible par machine : <https://bobine.fit/.well-known/security.txt>.

## Versions prises en charge

Seule la dernière release reçoit les correctifs de sécurité.

## Signaler une vulnérabilité

N'ouvrez pas de ticket public. Utilisez un de ces canaux privés :

1. **Signalement privé GitHub** : [onglet Security, « Report a vulnerability »](https://github.com/FantasmaGlad/bobine-deploy/security/advisories/new).
2. **Courriel** : [security@bobine.fit](mailto:security@bobine.fit), chiffré avec la clé OpenPGP
   `23CA D324 C507 FB0F 97E6 AECA 6E4C 020E F8BD FEB2` (`gpg --locate-keys security@bobine.fit`).

Accusé de réception sous 7 jours, divulgation coordonnée après 90 jours par défaut.

## Périmètre

Dans le périmètre : cette application et ses paquets, en particulier la vérification des clés d'hôte SSH, le traitement
des identifiants (mots de passe et clés privées effacés de la mémoire après usage, jamais journalisés), la découverte
réseau, la vérification de l'`install.sh` embarqué et la chaîne de signature des releases. Une faille de Bobine lui-même
se signale sur le dépôt de Bobine.

## Vérifier ce que nous publions

Chaque release porte `SHA256SUMS` et sa signature détachée `SHA256SUMS.asc`, faite avec la clé de publication de
Bobine, la même que celle des dépôts de paquets :

```bash
curl -fsSL https://packages.bobine.fit/bobine.gpg | gpg --show-keys --fingerprint
# clé principale : B77D 96D7 2F84 F36A CAA3  F872 9CBF 497D 829E E15A
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum --check --ignore-missing SHA256SUMS
```

L'`install.sh` embarqué provient d'une release signée de Bobine : son empreinte est épinglée dans `bobine-release.toml`
et contrôlée à la compilation.
