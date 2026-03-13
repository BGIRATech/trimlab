# TrimLab — Guide d'installation Windows 11

## 1. Rust

Télécharger et exécuter l'installeur officiel :
https://win.rustup.rs/x86_64

→ Choisir "1) Proceed with standard installation"
→ Redémarrer le terminal après l'installation

Vérifier :

```
rustc --version
cargo --version
```

---

## 2. Node.js

Télécharger la version LTS (20.x) :
https://nodejs.org/en/download

→ Installer avec les options par défaut
→ Cocher "Automatically install necessary tools" si proposé

Vérifier :

```
node --version
npm --version
```

---

## 3. Visual C++ Build Tools (requis par Rust sur Windows)

Télécharger Visual Studio Build Tools :
https://visualstudio.microsoft.com/visual-cpp-build-tools/

→ Cocher "Desktop development with C++"
→ Installer (environ 5 Go)

Sans ça, `cargo build` échouera avec des erreurs de linker.

---

## 4. WebView2 (requis par Tauri)

Déjà installé sur Windows 11. Rien à faire.

Vérifier dans : Paramètres → Applications → rechercher "WebView2"

---

## 5. FFmpeg (requis pour l'analyse audio/vidéo)

Télécharger le build statique Windows :
https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip

→ Extraire le zip, par exemple dans C:\ffmpeg\
→ Ajouter C:\ffmpeg\bin au PATH système :
Paramètres → Système → Variables d'environnement
→ PATH → Nouveau → C:\ffmpeg\bin
→ Redémarrer le terminal

Vérifier :

```
ffmpeg -version
ffprobe -version
```

---

## 6. Tauri CLI

```
cargo install tauri-cli --version "^2.0.0"
```

Cette commande prend 5 à 10 minutes la première fois.

Vérifier :

```
cargo tauri --version
```

---

## 7. Lancer TrimLab

```
cd trimlab
npm install
npm run tauri dev
```

La première compilation Rust prend 3 à 5 minutes.
Les compilations suivantes sont rapides grâce au cache Cargo.

---

## 8. Tester sans Rust (frontend seul)

Si tu veux tester l'interface sans compiler Rust :

```
cd trimlab
npm install
npm run dev
```

Ouvrir http://localhost:1420 dans le navigateur.

Le fichier `src/lib/commands.ts` détecte automatiquement l'absence de Tauri
et bascule sur des données mock. L'app est entièrement fonctionnelle en mode démo.

Clé démo pour débloquer l'export : AUTOTRIM-LIFETIME-DEMO

---

## Récapitulatif des versions

| Outil            | Version minimale | Vérification          |
| ---------------- | ---------------- | --------------------- |
| Rust             | 1.80+            | rustc --version       |
| Node.js          | 18+              | node --version        |
| npm              | 9+               | npm --version         |
| FFmpeg           | 6.x              | ffmpeg -version       |
| Tauri CLI        | 2.0+             | cargo tauri --version |
| WebView2         | any              | Déjà sur Win 11       |
| MSVC Build Tools | 2022             | Vérifier via VS       |
