mod analisis;
mod herramientas;

use tauri_plugin_opener::OpenerExt;

/// Abre una carpeta en el Explorador, creándola si todavía no existe.
#[tauri::command]
fn abrir_carpeta(app: tauri::AppHandle, ruta: String) -> Result<(), String> {
    std::fs::create_dir_all(&ruta).map_err(|e| format!("No se pudo crear la carpeta: {e}"))?;
    app.opener()
        .open_path(&ruta, None::<&str>)
        .map_err(|e| format!("No se pudo abrir la carpeta: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(herramientas::CandadoHerramientas::default())
        .manage(analisis::AnalisisEnCurso::default())
        .invoke_handler(tauri::generate_handler![
            abrir_carpeta,
            analisis::analizar_enlace,
            analisis::cancelar_analisis,
            herramientas::estado_herramientas,
            herramientas::instalar_herramientas,
            herramientas::actualizar_ytdlp,
            herramientas::actualizar_ffmpeg,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
