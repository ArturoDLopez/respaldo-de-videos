mod herramientas;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(herramientas::CandadoHerramientas::default())
        .invoke_handler(tauri::generate_handler![
            herramientas::estado_herramientas,
            herramientas::instalar_herramientas,
            herramientas::actualizar_ytdlp,
            herramientas::actualizar_ffmpeg,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación");
}
