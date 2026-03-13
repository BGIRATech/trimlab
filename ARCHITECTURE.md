# 🗂️ TrimLab — Carte complète des fichiers

## Vue d'ensemble : 2 grandes zones

```
trimlab/
│
├── 🟡 FRONTEND (ce que l'utilisateur voit — Svelte/JS)
│   └── src/
│
└── 🦀 BACKEND (le moteur — Rust)
    └── src-tauri/
```

---

## 🟡 FRONTEND — dossier `src/`

```
src/
│
├── main.ts             ← Point d'entrée. Lance l'app. Ne pas toucher.
├── App.svelte          ← Le routeur. Choisit quelle page afficher.
├── app.css             ← Design system global (couleurs, boutons, fonts…)
│
├── routes/             ← Les 3 PAGES de l'application
│   ├── Landing.svelte      → Page d'accueil / marketing
│   ├── App.svelte          → L'éditeur principal (waveform, timeline…)
│   └── Dashboard.svelte    → Le backoffice (stats, revenus, users…)
│
├── components/         ← Les morceaux réutilisables dans les pages
│   ├── waveform/
│   │   └── Waveform.svelte     → Le canvas waveform interactif
│   ├── timeline/
│   │   └── Timeline.svelte     → La liste des segments (silences, fillers…)
│   ├── licence/
│   │   └── LicenceModal.svelte → La popup d'activation de licence
│   ├── ExportModal.svelte      → La popup d'export (paywall inclus)
│   └── Notifications.svelte    → Les toasts en bas à droite
│
└── lib/                ← La logique partagée (pas d'UI)
    ├── store.ts            → L'état global de l'app (données, projets…)
    ├── commands.ts         → Les appels vers Rust (ou mock si dev)
    └── utils.ts            → Fonctions utilitaires (formatDuration, etc.)
```

---

## 🦀 BACKEND — dossier `src-tauri/`

```
src-tauri/
│
├── tauri.conf.json     ← Config de l'app (nom, icône, taille fenêtre…)
├── Cargo.toml          ← Les dépendances Rust (comme package.json)
│
└── src/
    └── main.rs         ← Tout le code Rust :
                            - probe_media()       → lit les infos d'un fichier vidéo
                            - analyse_silence()   → détecte les silences via FFmpeg
                            - analyse_fillers()   → détecte les "euh", "hm"… (Phase 3)
                            - export_segments()   → exporte en FCPXML / XML / fichiers
                            - get_waveform_data() → génère les données pour la waveform
                            - validate_licence()  → vérifie une clé de licence
                            - get_machine_id()    → identifie la machine (activation)
                            - open_file_dialog()  → ouvre le sélecteur de fichier natif
```

---

## 📁 Fichiers racine (à la base du projet)

```
trimlab/
│
├── index.html          ← La page HTML de base. Ne contient presque rien.
├── package.json        ← Dépendances JS (Svelte, Tauri, Vite…)
├── vite.config.ts      ← Config du bundler frontend
├── svelte.config.js    ← Config Svelte
├── tsconfig.json       ← Config TypeScript
└── README.md           ← Documentation du projet
```

---

## 🔁 Comment les fichiers communiquent entre eux

```
Utilisateur clique
       ↓
  routes/App.svelte          ← Page principale
       ↓
  components/Waveform.svelte  ← Affiche la waveform
       ↓
  lib/commands.ts             ← "Dis à Rust d'analyser ce fichier"
       ↓
  src-tauri/src/main.rs       ← Rust lance FFmpeg, retourne les segments
       ↓
  lib/store.ts                ← Stocke les segments dans l'état global
       ↓
  components/Timeline.svelte  ← Affiche les segments à l'écran
```

---

## 🚀 Ordre de lecture si tu débutes

1. **`src/lib/store.ts`** → Comprendre les données (types, état)
2. **`src/lib/commands.ts`** → Comprendre comment Svelte parle à Rust
3. **`src/routes/App.svelte`** → L'éditeur (logique principale)
4. **`src-tauri/src/main.rs`** → Le moteur Rust
5. **`src/routes/Landing.svelte`** → La landing page
6. **`src/routes/Dashboard.svelte`** → Le backoffice

---

## 📦 Pour lancer le projet

```bash
# Prérequis : Node.js + Rust + FFmpeg installés

npm install              # Installe les dépendances JS

npm run tauri dev        # Lance l'app en mode développement
                         # (hot-reload Svelte + Rust compilé)

npm run tauri build      # Build final (crée l'installeur .dmg / .exe / .deb)
```

> **Sans Rust installé ?** Le frontend seul fonctionne dans le navigateur
> grâce aux mocks dans `lib/commands.ts`.
> Clé de démo : `AUTOTRIM-LIFETIME-DEMO`
