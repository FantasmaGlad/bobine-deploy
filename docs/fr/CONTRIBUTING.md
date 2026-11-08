# Contribuer

Version anglaise : [CONTRIBUTING.md](../../CONTRIBUTING.md).

## Avant de commencer

- Ouvrez un ticket pour discuter de tout changement plus large qu'une correction.
- Une vulnérabilité ne va jamais dans un ticket public : voir [SECURITY.md](SECURITY.md).
- Le projet suit le code de conduite de Bobine : [CODE_OF_CONDUCT.md](../../CODE_OF_CONDUCT.md).

## Accord de licence des contributeurs

Bobine Deploy est distribué sous AGPL-3.0 ou ultérieure et peut aussi être proposé sous une licence commerciale
distincte. Pour le permettre, toute contribution suppose d'accepter l'[accord de licence des contributeurs](../../CLA.md),
en écrivant dans la demande de fusion : « I have read and agree to the CLA in CLA.md. » Une demande sans cette mention
n'est pas fusionnée.

## Contrôles

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Commits

Conventional Commits, en minuscules, courts : `type(scope): description`, par exemple
`fix(ssh): refuser une clé d'hôte changée`.
