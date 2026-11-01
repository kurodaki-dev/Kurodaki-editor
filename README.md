# Kurodaki Editor

Éditeur de design à calques écrit en Rust (modèle, rendu logiciel `tiny-skia`, interface `egui`).
Projet original : aucun code ni ressource d'une application tierce n'y est repris.

- `core/` : document (pages, calques rectangle/ellipse/image), rendu, export PNG, sauvegarde JSON.
- `app/` : interface egui, cible Android (`cargo apk`) et desktop.

```bash
cargo test -p kurodaki-core
cargo apk build --release -p kurodaki-editor --lib   # APK Android
```

Chaque push sur `main` compile l'APK avec GitHub Actions et publie une release.
L'APK est signé avec la clé de debug de `cargo-apk` ; ajoute ta propre keystore pour une diffusion publique.
