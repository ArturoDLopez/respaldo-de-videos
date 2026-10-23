//! Análisis de enlaces y búsqueda: averigua si un enlace es un video, una
//! playlist o un canal (o busca en YouTube) y lista sus videos, marcando los
//! que ya están respaldados.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};
use tokio::sync::Notify;

use crate::herramientas::{comando_ytdlp, traducir_error_ytdlp};

/// Archivo donde yt-dlp anota lo que ya descargó (`--download-archive`). Vive
/// en la carpeta de destino, así cada carpeta sabe qué tiene respaldado.
pub const ARCHIVO_DESCARGADOS: &str = ".respaldo-de-videos.txt";

/// Permite cancelar el análisis o la búsqueda en curso. La clave es
/// "analisis" o "busqueda", así una no cancela la otra.
#[derive(Default)]
pub struct ConsultasEnCurso(Mutex<HashMap<String, Arc<Notify>>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analisis {
    /// "video", "playlist" o "canal".
    tipo: &'static str,
    titulo: String,
    elementos: Vec<Elemento>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Elemento {
    id: String,
    titulo: String,
    url: String,
    /// Pestaña del canal de donde viene ("Videos", "Shorts", "Live"…).
    seccion: Option<String>,
    canal: Option<String>,
    vistas: Option<u64>,
    duracion: Option<f64>,
    miniatura: Option<String>,
    /// "disponible", "respaldado", "en_vivo" o "programado".
    estado: &'static str,
}

/// Acepta enlaces http(s) y, por comodidad, un `@canal` de YouTube suelto.
fn normalizar_enlace(enlace: &str) -> Result<String, String> {
    let enlace = enlace.trim();
    if enlace.starts_with('@') && !enlace.contains(char::is_whitespace) {
        return Ok(format!("https://www.youtube.com/{enlace}"));
    }
    if enlace.starts_with("https://") || enlace.starts_with("http://") {
        return Ok(enlace.to_string());
    }
    Err("Pega un enlace completo que empiece con https://".to_string())
}

pub fn leer_descargados(carpeta: &Path) -> HashSet<String> {
    std::fs::read_to_string(carpeta.join(ARCHIVO_DESCARGADOS))
        .map(|texto| texto.lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

fn texto(valor: &Value, clave: &str) -> Option<String> {
    valor.get(clave)?.as_str().map(str::to_string)
}

fn miniatura(entrada: &Value, extractor: &str, id: &str) -> Option<String> {
    // Para YouTube, la miniatura mediana (320×180) carga rápido aunque haya miles.
    if extractor.eq_ignore_ascii_case("youtube") {
        return Some(format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg"));
    }
    texto(entrada, "thumbnail").or_else(|| {
        entrada
            .get("thumbnails")?
            .as_array()?
            .last()
            .and_then(|t| texto(t, "url"))
    })
}

/// Recorre las entradas, entrando en las playlists anidadas (las pestañas de
/// un canal), y agrega cada video una sola vez.
fn aplanar(
    entradas: &[Value],
    seccion: Option<&str>,
    titulo_padre: &str,
    descargados: &HashSet<String>,
    vistos: &mut HashSet<String>,
    elementos: &mut Vec<Elemento>,
) {
    for entrada in entradas {
        if let Some(hijas) = entrada.get("entries").and_then(Value::as_array) {
            // "NASA - Shorts" → "Shorts"
            let titulo = texto(entrada, "title").unwrap_or_default();
            let nombre = titulo
                .strip_prefix(&format!("{titulo_padre} - "))
                .unwrap_or(&titulo)
                .to_string();
            aplanar(hijas, Some(&nombre), titulo_padre, descargados, vistos, elementos);
            continue;
        }

        let Some(id) = texto(entrada, "id") else { continue };
        let extractor = texto(entrada, "ie_key")
            .or_else(|| texto(entrada, "extractor_key"))
            .unwrap_or_default();
        if !vistos.insert(format!("{extractor} {id}")) {
            continue;
        }

        let estado = match entrada.get("live_status").and_then(Value::as_str) {
            Some("is_live") => "en_vivo",
            Some("is_upcoming") => "programado",
            _ if descargados.contains(&format!("{} {id}", extractor.to_lowercase())) => "respaldado",
            _ => "disponible",
        };

        elementos.push(Elemento {
            titulo: texto(entrada, "title").unwrap_or_else(|| id.clone()),
            url: texto(entrada, "webpage_url")
                .or_else(|| texto(entrada, "url"))
                .unwrap_or_default(),
            seccion: seccion.map(str::to_string),
            canal: texto(entrada, "channel").or_else(|| texto(entrada, "uploader")),
            vistas: entrada.get("view_count").and_then(Value::as_u64),
            duracion: entrada.get("duration").and_then(Value::as_f64),
            miniatura: miniatura(entrada, &extractor, &id),
            estado,
            id,
        });
    }
}

fn interpretar(info: &Value, descargados: &HashSet<String>) -> Analisis {
    let titulo = texto(info, "title").unwrap_or_default();
    let mut elementos = Vec::new();

    let Some(entradas) = info.get("entries").and_then(Value::as_array) else {
        aplanar(std::slice::from_ref(info), None, &titulo, descargados, &mut HashSet::new(), &mut elementos);
        return Analisis { tipo: "video", titulo, elementos };
    };

    let id = texto(info, "id").unwrap_or_default();
    let es_canal = id.starts_with('@') || texto(info, "channel_id").as_deref() == Some(id.as_str());
    let titulo_canal = texto(info, "channel").unwrap_or_else(|| titulo.clone());
    aplanar(entradas, None, &titulo_canal, descargados, &mut HashSet::new(), &mut elementos);

    Analisis {
        tipo: if es_canal { "canal" } else { "playlist" },
        titulo,
        elementos,
    }
}

/// Corre `yt-dlp --flat-playlist -J` sobre `objetivo` (un enlace o una
/// búsqueda "ytsearchN:…") y devuelve el JSON. Se puede cancelar con `clave`.
async fn consultar(
    app: &AppHandle,
    en_curso: &ConsultasEnCurso,
    clave: &str,
    objetivo: &str,
) -> Result<Value, String> {
    let cancelar = Arc::new(Notify::new());
    en_curso.0.lock().unwrap().insert(clave.to_string(), cancelar.clone());

    let hijo = comando_ytdlp(app)?
        .args(["--flat-playlist", "-J", "--"])
        .arg(objetivo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("No se pudo iniciar yt-dlp: {e}"))?;

    // Si se cancela, se suelta el proceso y kill_on_drop lo termina.
    let salida = tokio::select! {
        salida = hijo.wait_with_output() => salida.map_err(|e| format!("yt-dlp falló: {e}"))?,
        _ = cancelar.notified() => return Err("Cancelado.".to_string()),
    };
    en_curso.0.lock().unwrap().remove(clave);

    if !salida.status.success() {
        return Err(traducir_error_ytdlp(&String::from_utf8_lossy(&salida.stderr)));
    }
    serde_json::from_slice(&salida.stdout)
        .map_err(|e| format!("No se entendió la respuesta de yt-dlp: {e}"))
}

#[tauri::command]
pub async fn analizar_enlace(
    app: AppHandle,
    en_curso: State<'_, ConsultasEnCurso>,
    enlace: String,
    carpeta: String,
) -> Result<Analisis, String> {
    let enlace = normalizar_enlace(&enlace)?;
    let info = consultar(&app, &en_curso, "analisis", &enlace).await?;
    Ok(interpretar(&info, &leer_descargados(Path::new(&carpeta))))
}

/// Busca en YouTube sin API key, con la búsqueda integrada de yt-dlp.
#[tauri::command]
pub async fn buscar_videos(
    app: AppHandle,
    en_curso: State<'_, ConsultasEnCurso>,
    texto: String,
    cantidad: u32,
    carpeta: String,
) -> Result<Vec<Elemento>, String> {
    let texto = texto.trim();
    if texto.is_empty() {
        return Err("Escribe qué quieres buscar.".to_string());
    }
    let objetivo = format!("ytsearch{}:{texto}", cantidad.clamp(1, 100));
    let info = consultar(&app, &en_curso, "busqueda", &objetivo).await?;
    Ok(interpretar(&info, &leer_descargados(Path::new(&carpeta))).elementos)
}

#[tauri::command]
pub fn cancelar_consulta(en_curso: State<'_, ConsultasEnCurso>, clave: String) {
    if let Some(cancelar) = en_curso.0.lock().unwrap().remove(&clave) {
        cancelar.notify_one();
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    fn video(id: &str, estado_vivo: Option<&str>) -> Value {
        json!({ "_type": "url", "ie_key": "Youtube", "id": id, "title": format!("Video {id}"),
                "url": format!("https://www.youtube.com/watch?v={id}"), "duration": 60.0,
                "live_status": estado_vivo })
    }

    #[test]
    fn aplana_las_pestanas_de_un_canal() {
        let info = json!({
            "_type": "playlist", "id": "@Canal", "title": "Canal", "channel": "Canal",
            "channel_id": "UC123",
            "entries": [
                { "_type": "playlist", "title": "Canal - Videos",
                  "entries": [video("aaa", None), video("BbB", None)] },
                { "_type": "playlist", "title": "Canal - Live",
                  "entries": [video("aaa", None), video("ccc", Some("is_upcoming"))] },
            ]
        });
        let descargados = HashSet::from(["youtube BbB".to_string()]);
        let analisis = interpretar(&info, &descargados);

        assert_eq!(analisis.tipo, "canal");
        let resumen: Vec<_> = analisis
            .elementos
            .iter()
            .map(|e| (e.id.as_str(), e.seccion.as_deref(), e.estado))
            .collect();
        assert_eq!(
            resumen,
            [
                ("aaa", Some("Videos"), "disponible"),
                ("BbB", Some("Videos"), "respaldado"),
                ("ccc", Some("Live"), "programado"),
            ]
        );
    }

    #[test]
    fn reconoce_un_video_suelto() {
        let info = json!({ "_type": "video", "id": "xyz", "title": "Hola", "extractor_key": "Youtube",
                           "webpage_url": "https://www.youtube.com/watch?v=xyz", "url": "https://media/xyz.mp4" });
        let analisis = interpretar(&info, &HashSet::new());
        assert_eq!(analisis.tipo, "video");
        assert_eq!(analisis.elementos[0].url, "https://www.youtube.com/watch?v=xyz");
    }

    #[test]
    fn distingue_playlist_de_canal() {
        let info = json!({ "_type": "playlist", "id": "PL123", "title": "Mi lista", "channel_id": "UC123",
                           "entries": [video("aaa", None)] });
        assert_eq!(interpretar(&info, &HashSet::new()).tipo, "playlist");
    }

    #[test]
    fn normaliza_enlaces() {
        assert_eq!(normalizar_enlace(" @Canal ").unwrap(), "https://www.youtube.com/@Canal");
        assert!(normalizar_enlace("--exec calc").is_err());
        assert!(normalizar_enlace("hola mundo").is_err());
    }

    #[test]
    fn traduce_errores_de_ytdlp() {
        let stderr = "[youtube] abc: Downloading webpage\nERROR: [youtube] abc: Private video. Sign in";
        assert!(traducir_error_ytdlp(stderr).starts_with("El video es privado."));
    }
}
