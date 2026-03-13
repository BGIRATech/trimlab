// ============================================================
// FICHIER : trimlab/src-tauri/build.rs
// ROLE    : Script de build requis par tauri::generate_context!()
//           Sans ce fichier, OUT_DIR n'est pas defini et la macro plante
// ============================================================
fn main() {
    tauri_build::build()
}