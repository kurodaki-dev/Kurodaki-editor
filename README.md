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

## 100 % local
- Aucune permission `INTERNET` : l'app ne communique avec aucun serveur, tout se passe sur l'appareil.
- **Import** : photos de `Pictures`, `DCIM/Camera`, `Download` et `Kurodaki` (autorise l'accès aux photos dans les réglages Android).
- **Magic Touch** : amélioration automatique (niveaux + couleurs) calculée en Rust, plus N&B, sépia, contraste, lumière, couleurs.
- **Retirer le fond (IA locale)** : exécute un modèle ONNX avec `tract`. Place toi-même un modèle compatible U²-Net
  (ex. `u2netp.onnx`, entrée 320×320) dans `Kurodaki/models/u2netp.onnx` ; il n'est jamais téléchargé par l'app.
- Projets enregistrés en JSON dans `Kurodaki/projet.kuro.json`, exports dans `Kurodaki/export.png`.
