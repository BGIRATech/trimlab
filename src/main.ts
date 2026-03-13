// ============================================================
// FICHIER : trimlab/src/main.ts
// ROLE    : Point d'entrée Svelte — monte App.svelte dans #app
// DÉCLENCHÉ PAR : index.html <script src='/src/main.ts'>
// ============================================================
import App from './App.svelte'
import './app.css'

const app = new App({
  target: document.getElementById('app')!,
})

export default app
