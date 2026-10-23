//! Descarga, verificación y actualización de yt-dlp y ffmpeg.
//!
//! Las herramientas no van dentro del instalador: se descargan la primera vez
//! que se abre la app y se guardan en `%LOCALAPPDATA%\<identificador>\bin`.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

const URL_YTDLP: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
const URL_YTDLP_SUMAS: &str =
    "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";
const URL_YTDLP_ULTIMA: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";

const ZIP_FFMPEG: &str = "ffmpeg-master-latest-win64-gpl.zip";
const URL_FFMPEG: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
const URL_FFMPEG_SUMAS: &str =
    "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/checksums.sha256";

const EVENTO_PROGRESO: &str = "herramientas-progreso";

/// Evita que dos instalaciones o actualizaciones corran a la vez.
#[derive(Default)]
pub struct CandadoHerramientas(Mutex<()>);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EstadoHerramientas {
    /// Versión instalada, o `None` si falta.
    ytdlp: Option<String>,
    ffmpeg: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progreso {
    herramienta: &'static str,
    /// "descargando", "verificando", "extrayendo" o "listo".
    etapa: &'static str,
    descargado: u64,
    total: Option<u64>,
}

pub fn carpeta_bin(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|d| d.join("bin"))
        .map_err(|e| format!("No se encontró la carpeta de datos de la app: {e}"))
}

pub fn ruta_ytdlp(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(carpeta_bin(app)?.join("yt-dlp.exe"))
}

pub fn ruta_ffmpeg(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(carpeta_bin(app)?.join("ffmpeg.exe"))
}

/// Crea un comando que no abre una ventana de consola en Windows.
pub fn comando_oculto(programa: &Path) -> tokio::process::Command {
    let mut comando = tokio::process::Command::new(programa);
    #[cfg(windows)]
    comando.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    comando
}

/// Prepara yt-dlp con las opciones comunes: sin consola, en UTF-8, sin leer
/// configuraciones globales del usuario y usando nuestro ffmpeg.
pub fn comando_ytdlp(app: &AppHandle) -> Result<tokio::process::Command, String> {
    let mut comando = comando_oculto(&ruta_ytdlp(app)?);
    comando
        .env("PYTHONUTF8", "1")
        .arg("--ignore-config")
        .arg("--no-warnings")
        // Usa los certificados de Windows en vez de los propios de yt-dlp: así
        // funciona con antivirus o proxies que revisan HTTPS (p. ej. Avast).
        .args(["--compat-options", "no-certifi"])
        .arg("--ffmpeg-location")
        .arg(carpeta_bin(app)?);
    Ok(comando)
}

/// Convierte el último "ERROR:" de yt-dlp en un mensaje en español.
pub fn traducir_error_ytdlp(stderr: &str) -> String {
    let original = stderr
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("ERROR:"))
        .map(str::trim)
        .unwrap_or_else(|| stderr.trim());
    let minusculas = original.to_lowercase();

    // Se usa la primera coincidencia, así que los casos más específicos van antes.
    let traduccion = [
        // Enlaces y búsqueda
        ("is not a valid url", "Eso no parece un enlace válido."),
        ("unsupported url", "Ese enlace no es compatible. Pega el enlace de un video, una playlist o un canal."),
        ("this channel does not exist", "El canal no existe."),
        ("does not have a", "El canal no tiene esa sección."),
        ("http error 404", "No se encontró la página (error 404)."),
        // Videos que no se pueden descargar
        ("private video", "El video es privado. Si es tuyo, cámbialo a \"No listado\" mientras lo respaldas."),
        ("members-only", "El video es solo para miembros del canal."),
        ("join this channel", "El video es solo para miembros del canal."),
        ("sign in to confirm your age", "El video tiene restricción de edad y YouTube pide iniciar sesión."),
        ("not a bot", "YouTube pidió confirmar que no eres un robot. Espera un rato e intenta de nuevo."),
        ("copyright", "YouTube retiró el video por un reclamo de derechos de autor."),
        ("removed by the uploader", "El video fue eliminado por quien lo subió."),
        ("account associated with this video has been terminated", "La cuenta que subió el video fue cerrada."),
        ("not available in your country", "El video no está disponible en tu país."),
        ("geo restriction", "El video no está disponible en tu país."),
        ("premieres in", "El video es un estreno que todavía no empieza."),
        ("live event will begin", "La transmisión todavía no empieza."),
        ("this live event has ended", "La transmisión terminó y todavía no está disponible como video."),
        ("requested format is not available", "No hay una versión del video con la calidad elegida. Prueba otra calidad en Ajustes."),
        ("video unavailable", "El video no está disponible."),
        // YouTube cambió algo: casi siempre se arregla actualizando yt-dlp
        ("http error 403", "YouTube rechazó la descarga (error 403). Actualiza yt-dlp en Ajustes e intenta de nuevo."),
        ("unable to extract", "YouTube cambió algo que yt-dlp no entiende. Actualiza yt-dlp en Ajustes e intenta de nuevo."),
        ("http error 429", "YouTube está limitando las peticiones. Espera unos minutos e intenta de nuevo."),
        // Disco y archivos
        ("no space left", "No hay espacio suficiente en el disco."),
        ("errno 28", "No hay espacio suficiente en el disco."),
        ("permission denied", "No hay permiso para escribir en la carpeta de destino."),
        ("errno 13", "No hay permiso para escribir en la carpeta de destino."),
        ("filename or extension is too long", "La ruta del archivo es demasiado larga. Elige una carpeta más corta o el formato \"Solo el título\"."),
        ("ffmpeg not found", "No se encontró ffmpeg. Actualízalo en Ajustes."),
        ("ffprobe and ffmpeg not found", "No se encontró ffmpeg. Actualízalo en Ajustes."),
        // Conexión
        ("certificate verify failed", "No se pudo verificar la conexión segura. Si usas un antivirus o un proxy que revisa HTTPS, revisa su configuración."),
        ("getaddrinfo failed", "No hay conexión a internet."),
        ("connection reset", "Se cortó la conexión. Intenta de nuevo."),
        ("timed out", "La conexión tardó demasiado. Revisa tu internet e intenta de nuevo."),
    ]
    .iter()
    .find(|(clave, _)| minusculas.contains(clave))
    .map(|(_, texto)| *texto);

    match traduccion {
        Some(texto) => texto.to_string(),
        None if original.is_empty() => "yt-dlp falló sin dar detalles.".to_string(),
        None => format!("yt-dlp no pudo completar la operación: {original}"),
    }
}

/// Ejecuta el programa y devuelve la primera línea de su salida, o `None` si
/// no existe o falla.
async fn primera_linea(programa: &Path, argumento: &str) -> Option<String> {
    if !programa.exists() {
        return None;
    }
    let salida = comando_oculto(programa)
        .arg(argumento)
        .output()
        .await
        .ok()?;
    if !salida.status.success() {
        return None;
    }
    let texto = String::from_utf8_lossy(&salida.stdout);
    texto.lines().next().map(|l| l.trim().to_string())
}

async fn version_ytdlp(app: &AppHandle) -> Result<Option<String>, String> {
    Ok(primera_linea(&ruta_ytdlp(app)?, "--version").await)
}

async fn version_ffmpeg(app: &AppHandle) -> Result<Option<String>, String> {
    if !carpeta_bin(app)?.join("ffprobe.exe").exists() {
        return Ok(None);
    }
    // "ffmpeg version N-121234-g1a2b3c4d5e-20261020 Copyright (c) ..."
    Ok(primera_linea(&ruta_ffmpeg(app)?, "-version")
        .await
        .and_then(|l| l.split_whitespace().nth(2).map(str::to_string)))
}

async fn leer_estado(app: &AppHandle) -> Result<EstadoHerramientas, String> {
    Ok(EstadoHerramientas {
        ytdlp: version_ytdlp(app).await?,
        ffmpeg: version_ffmpeg(app).await?,
    })
}

fn cliente() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("RespaldoDeVideos/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("No se pudo preparar la conexión: {e}"))
}

fn error_red(e: reqwest::Error) -> String {
    format!("No se pudo descargar. Revisa tu conexión a internet. ({e})")
}

fn a_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Busca en un archivo de sumas (formato `hash  nombre`) la suma de `nombre`.
async fn suma_esperada(
    cliente: &reqwest::Client,
    url: &str,
    nombre: &str,
) -> Result<String, String> {
    let texto = cliente
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(error_red)?
        .text()
        .await
        .map_err(error_red)?;
    texto
        .lines()
        .find_map(|linea| {
            let mut partes = linea.split_whitespace();
            let hash = partes.next()?;
            (partes.next()?.trim_start_matches('*') == nombre).then(|| hash.to_lowercase())
        })
        .ok_or_else(|| format!("No se encontró la suma de verificación de {nombre}."))
}

/// Descarga `url` en `destino`, avisando el progreso, y devuelve su SHA-256.
async fn descargar(
    app: &AppHandle,
    cliente: &reqwest::Client,
    herramienta: &'static str,
    url: &str,
    destino: &Path,
) -> Result<String, String> {
    let respuesta = cliente
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(error_red)?;
    let total = respuesta.content_length();

    let mut archivo = tokio::fs::File::create(destino)
        .await
        .map_err(|e| format!("No se pudo crear {}: {e}", destino.display()))?;
    let mut hasher = Sha256::new();
    let mut descargado = 0u64;
    let mut ultimo_aviso = Instant::now();
    let mut flujo = respuesta.bytes_stream();

    while let Some(trozo) = flujo.next().await {
        let trozo = trozo.map_err(error_red)?;
        hasher.update(&trozo);
        archivo
            .write_all(&trozo)
            .await
            .map_err(|e| format!("No se pudo guardar la descarga: {e}"))?;
        descargado += trozo.len() as u64;

        if ultimo_aviso.elapsed() >= Duration::from_millis(200) {
            ultimo_aviso = Instant::now();
            avisar(app, herramienta, "descargando", descargado, total);
        }
    }
    archivo
        .flush()
        .await
        .map_err(|e| format!("No se pudo guardar la descarga: {e}"))?;
    avisar(app, herramienta, "verificando", descargado, total);

    Ok(a_hex(&hasher.finalize()))
}

fn avisar(
    app: &AppHandle,
    herramienta: &'static str,
    etapa: &'static str,
    descargado: u64,
    total: Option<u64>,
) {
    let _ = app.emit(
        EVENTO_PROGRESO,
        Progreso {
            herramienta,
            etapa,
            descargado,
            total,
        },
    );
}

fn verificar(obtenida: &str, esperada: &str, temporal: &Path, nombre: &str) -> Result<(), String> {
    if obtenida == esperada {
        return Ok(());
    }
    let _ = std::fs::remove_file(temporal);
    Err(format!(
        "La descarga de {nombre} llegó dañada (la suma SHA-256 no coincide). Intenta de nuevo."
    ))
}

async fn preparar_carpeta(app: &AppHandle) -> Result<PathBuf, String> {
    let bin = carpeta_bin(app)?;
    tokio::fs::create_dir_all(&bin)
        .await
        .map_err(|e| format!("No se pudo crear la carpeta {}: {e}", bin.display()))?;
    Ok(bin)
}

async fn instalar_ytdlp(app: &AppHandle, cliente: &reqwest::Client) -> Result<(), String> {
    let bin = preparar_carpeta(app).await?;
    let esperada = suma_esperada(cliente, URL_YTDLP_SUMAS, "yt-dlp.exe").await?;
    let temporal = bin.join("yt-dlp.exe.part");

    let obtenida = descargar(app, cliente, "ytdlp", URL_YTDLP, &temporal).await?;
    verificar(&obtenida, &esperada, &temporal, "yt-dlp")?;

    tokio::fs::rename(&temporal, bin.join("yt-dlp.exe"))
        .await
        .map_err(|e| format!("No se pudo reemplazar yt-dlp (¿hay una descarga en curso?): {e}"))?;
    avisar(app, "ytdlp", "listo", 0, None);
    Ok(())
}

async fn instalar_ffmpeg(app: &AppHandle, cliente: &reqwest::Client) -> Result<(), String> {
    let bin = preparar_carpeta(app).await?;
    let esperada = suma_esperada(cliente, URL_FFMPEG_SUMAS, ZIP_FFMPEG).await?;
    let zip = bin.join("ffmpeg.zip.part");

    let obtenida = descargar(app, cliente, "ffmpeg", URL_FFMPEG, &zip).await?;
    verificar(&obtenida, &esperada, &zip, "ffmpeg")?;

    avisar(app, "ffmpeg", "extrayendo", 0, None);
    let resultado = {
        let zip = zip.clone();
        let bin = bin.clone();
        tauri::async_runtime::spawn_blocking(move || extraer_ffmpeg(&zip, &bin))
            .await
            .map_err(|e| format!("Falló la extracción de ffmpeg: {e}"))?
    };
    let _ = tokio::fs::remove_file(&zip).await;
    resultado?;

    avisar(app, "ffmpeg", "listo", 0, None);
    Ok(())
}

/// Saca `ffmpeg.exe` y `ffprobe.exe` de la carpeta `bin/` dentro del zip.
fn extraer_ffmpeg(zip: &Path, bin: &Path) -> Result<(), String> {
    let error = |e: &dyn std::fmt::Display| format!("No se pudo extraer ffmpeg: {e}");
    let archivo = std::fs::File::open(zip).map_err(|e| error(&e))?;
    let mut zip = zip::ZipArchive::new(archivo).map_err(|e| error(&e))?;

    for nombre in ["ffmpeg.exe", "ffprobe.exe"] {
        let indice = (0..zip.len())
            .find(|&i| {
                matches!(zip.name_for_index(i), Some(Ok(n)) if n.ends_with(&format!("/bin/{nombre}")))
            })
            .ok_or_else(|| format!("El zip de ffmpeg no trae {nombre}."))?;

        let mut entrada = zip.by_index(indice).map_err(|e| error(&e))?;
        let mut contenido = Vec::with_capacity(entrada.size() as usize);
        entrada.read_to_end(&mut contenido).map_err(|e| error(&e))?;

        let temporal = bin.join(format!("{nombre}.part"));
        std::fs::write(&temporal, contenido).map_err(|e| error(&e))?;
        std::fs::rename(&temporal, bin.join(nombre)).map_err(|e| {
            format!("No se pudo reemplazar {nombre} (¿hay una descarga en curso?): {e}")
        })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn estado_herramientas(app: AppHandle) -> Result<EstadoHerramientas, String> {
    leer_estado(&app).await
}

/// Descarga las herramientas que falten.
#[tauri::command]
pub async fn instalar_herramientas(
    app: AppHandle,
    candado: State<'_, CandadoHerramientas>,
) -> Result<EstadoHerramientas, String> {
    let _guardia = candado.0.lock().await;
    let cliente = cliente()?;
    if version_ytdlp(&app).await?.is_none() {
        instalar_ytdlp(&app, &cliente).await?;
    }
    if version_ffmpeg(&app).await?.is_none() {
        instalar_ffmpeg(&app, &cliente).await?;
    }
    leer_estado(&app).await
}

/// Descarga yt-dlp si hay una versión más nueva. Devuelve la versión nueva, o
/// `None` si ya estaba al día.
#[tauri::command]
pub async fn actualizar_ytdlp(
    app: AppHandle,
    candado: State<'_, CandadoHerramientas>,
) -> Result<Option<String>, String> {
    let _guardia = candado.0.lock().await;
    let cliente = cliente()?;

    // La página de la última versión redirige a .../releases/tag/<versión>.
    let respuesta = cliente
        .head(URL_YTDLP_ULTIMA)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(error_red)?;
    let ultima = respuesta
        .url()
        .path_segments()
        .and_then(|mut s| s.next_back())
        .map(str::to_string)
        .ok_or("No se pudo saber cuál es la última versión de yt-dlp.")?;

    if version_ytdlp(&app).await?.as_deref() == Some(ultima.as_str()) {
        return Ok(None);
    }
    instalar_ytdlp(&app, &cliente).await?;
    Ok(version_ytdlp(&app).await?)
}

/// Vuelve a descargar ffmpeg (la compilación "latest" cambia seguido).
#[tauri::command]
pub async fn actualizar_ffmpeg(
    app: AppHandle,
    candado: State<'_, CandadoHerramientas>,
) -> Result<EstadoHerramientas, String> {
    let _guardia = candado.0.lock().await;
    instalar_ffmpeg(&app, &cliente()?).await?;
    leer_estado(&app).await
}

#[cfg(test)]
mod pruebas {
    use super::traducir_error_ytdlp;

    #[test]
    fn traduce_errores_comunes() {
        let casos = [
            ("ERROR: [youtube] abc: Video unavailable. This video has been removed by the uploader", "eliminado por quien lo subió"),
            ("ERROR: [youtube] abc: Requested format is not available. Use --list-formats", "calidad elegida"),
            ("ERROR: unable to download video data: HTTP Error 403: Forbidden", "error 403"),
            ("ERROR: [Errno 28] No space left on device", "espacio"),
            ("ERROR: [youtube] abc: Sign in to confirm you're not a bot", "robot"),
        ];
        for (stderr, esperado) in casos {
            let traducido = traducir_error_ytdlp(stderr);
            assert!(traducido.contains(esperado), "{stderr:?} → {traducido:?}");
        }
    }

    #[test]
    fn deja_el_original_si_no_lo_conoce() {
        assert_eq!(
            traducir_error_ytdlp("ERROR: algo raro"),
            "yt-dlp no pudo completar la operación: algo raro"
        );
    }
}
