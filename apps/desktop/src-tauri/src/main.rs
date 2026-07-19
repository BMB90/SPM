// Intentionally thin: the desktop shell hosts the React UI, which talks
// to `spm-api` over plain HTTP (see apps/desktop/src/api/client.ts). No
// Tauri commands are needed yet — this is the seam where native-only
// conveniences (system tray, native file dialogs for report export,
// auto-launching `spm-api` as a sidecar) would be added.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the SPM desktop application");
}
