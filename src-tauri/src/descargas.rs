//! Cola de descargas: corre yt-dlp para cada video, con un límite de
//! descargas a la vez, y avisa el progreso a la interfaz.

use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Notify;

use crate::analisis::ARCHIVO_DESCARGADOS;
use crate::herramientas::{comando_ytdlp, traducir_error_ytdlp};

const EVENTO_COLA: &str = "cola-actualizada";
const EVENTO_DESCARGA: &str = "descarga-actualizada";

#[derive(Deserialize, Clone, Copy)]
pub enum Calidad {
    #[serde(rename = "mejor")]
    Mejor,
    #[serde(rename = "1080")]
    Hasta1080,
    #[serde(rename = "720")]
    Hasta720,
    #[serde(rename = "audio")]
    Audio,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum Plantilla {
    CanalFecha,
    Fecha,
    Titulo,
}

/// Ajustes con los que se agregó cada video; cambiar los ajustes después no
/// afecta lo que ya está en la cola.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Opciones {
    carpeta: String,
    calidad: Calidad,
    plantilla: Plantilla,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuevaDescarga {
    video_id: String,
    titulo: String,
    url: String,
    miniatura: Option<String>,
    /// Ya estaba respaldado y se pidió descargarlo de nuevo.
    #[serde(default)]
    forzar: bool,
}

#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Estado {
    EnEspera,
    Descargando,
    Procesando,
    Terminado,
    Error,
    Cancelado,
}

impl Estado {
    fn activo(self) -> bool {
        matches!(self, Estado::Descargando | Estado::Procesando)
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Descarga {
    id: u64,
    video_id: String,
    titulo: String,
    url: String,
    miniatura: Option<String>,
    estado: Estado,
    /// De 0 a 100 dentro de la parte actual.
    porcentaje: Option<f64>,
    /// Bytes por segundo.
    velocidad: Option<f64>,
    /// Segundos restantes.
    eta: Option<f64>,
    /// Parte actual (video, audio…) y cuántas hay.
    parte: u32,
    partes: u32,
    /// Paso de ffmpeg en curso ("Merger", "EmbedThumbnail"…).
    proceso: Option<String>,
    archivo: Option<String>,
    nota: Option<String>,
    error: Option<String>,
}

struct Entrada {
    descarga: Descarga,
    opciones: Opciones,
    forzar: bool,
    cancelar: Option<Arc<Notify>>,
    pid: Option<u32>,
}

struct ColaInterna {
    entradas: Vec<Entrada>,
    siguiente_id: u64,
    maximo: usize,
}

pub struct Cola(Mutex<ColaInterna>);

impl Default for Cola {
    fn default() -> Self {
        Cola(Mutex::new(ColaInterna { entradas: Vec::new(), siguiente_id: 1, maximo: 2 }))
    }
}

impl ColaInterna {
    fn entrada(&mut self, id: u64) -> Option<&mut Entrada> {
        self.entradas.iter_mut().find(|e| e.descarga.id == id)
    }

    fn lista(&self) -> Vec<Descarga> {
        self.entradas.iter().map(|e| e.descarga.clone()).collect()
    }
}

enum Fallo {
    Cancelado,
    Mensaje(String),
}

fn avisar_cola(app: &AppHandle, cola: &ColaInterna) {
    let _ = app.emit(EVENTO_COLA, cola.lista());
}

/// Cambia una descarga y avisa a la interfaz.
fn actualizar(app: &AppHandle, id: u64, cambio: impl FnOnce(&mut Entrada)) {
    let cola = app.state::<Cola>();
    let mut cola = cola.0.lock().unwrap();
    if let Some(entrada) = cola.entrada(id) {
        cambio(entrada);
        let _ = app.emit(EVENTO_DESCARGA, entrada.descarga.clone());
    }
}

/// Arranca descargas en espera hasta llenar el límite.
fn programar(app: &AppHandle) {
    let cola = app.state::<Cola>();
    let mut cola = cola.0.lock().unwrap();
    let mut activas = cola.entradas.iter().filter(|e| e.descarga.estado.activo()).count();

    while activas < cola.maximo {
        let Some(entrada) = cola.entradas.iter_mut().find(|e| e.descarga.estado == Estado::EnEspera) else {
            break;
        };
        entrada.descarga.estado = Estado::Descargando;
        let cancelar = Arc::new(Notify::new());
        entrada.cancelar = Some(cancelar.clone());
        let _ = app.emit(EVENTO_DESCARGA, entrada.descarga.clone());

        let id = entrada.descarga.id;
        let url = entrada.descarga.url.clone();
        let opciones = entrada.opciones.clone();
        let forzar = entrada.forzar;
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let resultado = correr_ytdlp(&app, id, &url, &opciones, forzar, cancelar).await;
            actualizar(&app, id, |e| {
                e.cancelar = None;
                e.pid = None;
                e.descarga.velocidad = None;
                e.descarga.eta = None;
                e.descarga.proceso = None;
                match resultado {
                    Ok(Some(archivo)) => {
                        e.descarga.estado = Estado::Terminado;
                        e.descarga.porcentaje = Some(100.0);
                        e.descarga.archivo = Some(archivo);
                    }
                    Ok(None) => {
                        e.descarga.estado = Estado::Terminado;
                        e.descarga.porcentaje = Some(100.0);
                        e.descarga.nota = Some("Ya estaba respaldado en esta carpeta.".into());
                    }
                    Err(Fallo::Cancelado) => e.descarga.estado = Estado::Cancelado,
                    Err(Fallo::Mensaje(m)) => {
                        e.descarga.estado = Estado::Error;
                        e.descarga.error = Some(m);
                    }
                }
            });
            programar(&app);
        });
        activas += 1;
    }
}

fn plantilla_salida(plantilla: Plantilla) -> &'static str {
    // .150B recorta títulos largos para no pasar el límite de rutas de Windows.
    match plantilla {
        Plantilla::CanalFecha => {
            "%(channel,uploader|Sin canal)s/%(upload_date>%Y-%m-%d|Sin fecha)s - %(title).150B [%(id)s].%(ext)s"
        }
        Plantilla::Fecha => "%(upload_date>%Y-%m-%d|Sin fecha)s - %(title).150B [%(id)s].%(ext)s",
        Plantilla::Titulo => "%(title).150B [%(id)s].%(ext)s",
    }
}

fn argumentos_calidad(calidad: Calidad) -> &'static [&'static str] {
    // -S ordena por resolución y, a igual resolución, prefiere MP4/M4A para
    // no tener que convertir. El resultado siempre se une en MP4.
    match calidad {
        Calidad::Mejor => &["-f", "bv*+ba/b", "-S", "res,fps,ext:mp4:m4a", "--merge-output-format", "mp4"],
        Calidad::Hasta1080 => &["-f", "bv*+ba/b", "-S", "res:1080,fps,ext:mp4:m4a", "--merge-output-format", "mp4"],
        Calidad::Hasta720 => &["-f", "bv*+ba/b", "-S", "res:720,fps,ext:mp4:m4a", "--merge-output-format", "mp4"],
        Calidad::Audio => &["-f", "ba/b", "-x", "--audio-format", "m4a"],
    }
}

fn numero(valor: Option<&str>) -> Option<f64> {
    valor.and_then(|v| v.parse().ok())
}

/// Descarga un video. Devuelve la ruta final, o `None` si yt-dlp lo saltó
/// porque ya estaba en el archivo de descargados.
async fn correr_ytdlp(
    app: &AppHandle,
    id: u64,
    url: &str,
    opciones: &Opciones,
    forzar: bool,
    cancelar: Arc<Notify>,
) -> Result<Option<String>, Fallo> {
    let carpeta = Path::new(&opciones.carpeta);
    let mut comando = comando_ytdlp(app).map_err(Fallo::Mensaje)?;
    comando
        .args(["--newline", "--progress", "--no-playlist", "--no-simulate"])
        .args([
            "--progress-template",
            "download:PROGRESO %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s %(progress.speed)s %(progress.eta)s %(info.format_id)s",
            "--progress-template",
            "postprocess:PROCESANDO %(progress.postprocessor)s",
            "--print",
            "before_dl:FORMATO %(format_id)s",
            "--print",
            "after_move:FINAL %(filepath)s",
        ])
        .args(argumentos_calidad(opciones.calidad))
        .args(["--embed-metadata", "--embed-thumbnail", "--embed-chapters"])
        .arg("-P")
        .arg(carpeta)
        .arg("-o")
        .arg(plantilla_salida(opciones.plantilla));
    // Si ya estaba respaldado y se pidió de nuevo, no se consulta el archivo
    // de descargados (si no, yt-dlp lo saltaría).
    if !forzar {
        comando.arg("--download-archive").arg(carpeta.join(ARCHIVO_DESCARGADOS));
    }

    let mut hijo = comando
        .arg("--")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| Fallo::Mensaje(format!("No se pudo iniciar yt-dlp: {e}")))?;
    let pid = hijo.id();
    actualizar(app, id, |e| e.pid = pid);

    let mut salida = BufReader::new(hijo.stdout.take().unwrap()).lines();
    let mut errores = BufReader::new(hijo.stderr.take().unwrap()).lines();

    let mut archivo = None;
    let mut texto_errores = String::new();
    let mut formatos: Vec<String> = Vec::new();
    let mut ultimo_aviso = Instant::now() - Duration::from_secs(1);

    let lectura = async {
        let mut abierta_salida = true;
        let mut abierta_errores = true;
        while abierta_salida || abierta_errores {
            tokio::select! {
                linea = salida.next_line(), if abierta_salida => match linea {
                    Ok(Some(linea)) => {
                        if let Some(resto) = linea.strip_prefix("PROGRESO ") {
                            if ultimo_aviso.elapsed() < Duration::from_millis(250) {
                                continue;
                            }
                            ultimo_aviso = Instant::now();
                            let campos: Vec<&str> = resto.split(' ').collect();
                            let bajado = numero(campos.first().copied());
                            let total = numero(campos.get(1).copied()).or(numero(campos.get(2).copied()));
                            let parte = campos
                                .get(5)
                                .and_then(|f| formatos.iter().position(|x| x == f))
                                .map_or(1, |i| i as u32 + 1);
                            actualizar(app, id, |e| {
                                e.descarga.estado = Estado::Descargando;
                                e.descarga.porcentaje = bajado
                                    .zip(total)
                                    .filter(|(_, t)| *t > 0.0)
                                    .map(|(b, t)| (b / t * 100.0).min(100.0));
                                e.descarga.velocidad = numero(campos.get(3).copied());
                                e.descarga.eta = numero(campos.get(4).copied());
                                e.descarga.parte = parte;
                            });
                        } else if let Some(formato) = linea.strip_prefix("FORMATO ") {
                            formatos = formato.split('+').map(str::to_string).collect();
                            let partes = formatos.len() as u32;
                            actualizar(app, id, |e| {
                                e.descarga.parte = 1;
                                e.descarga.partes = partes;
                            });
                        } else if let Some(ruta) = linea.strip_prefix("FINAL ") {
                            archivo = Some(ruta.to_string());
                        }
                    }
                    _ => abierta_salida = false,
                },
                linea = errores.next_line(), if abierta_errores => match linea {
                    Ok(Some(linea)) => {
                        if let Some(paso) = linea.strip_prefix("PROCESANDO ") {
                            let paso = paso.to_string();
                            actualizar(app, id, |e| {
                                e.descarga.estado = Estado::Procesando;
                                e.descarga.proceso = Some(paso);
                                e.descarga.velocidad = None;
                                e.descarga.eta = None;
                            });
                        } else {
                            texto_errores.push_str(&linea);
                            texto_errores.push('\n');
                        }
                    }
                    _ => abierta_errores = false,
                },
            }
        }
        hijo.wait().await
    };

    let estado = tokio::select! {
        estado = lectura => estado,
        _ = cancelar.notified() => {
            // yt-dlp lanza ffmpeg como subproceso: hay que cerrar todo el árbol.
            if let Some(pid) = pid {
                matar_arbol(pid);
            }
            return Err(Fallo::Cancelado);
        }
    };

    match estado {
        Ok(s) if s.success() => Ok(archivo),
        Ok(_) => Err(Fallo::Mensaje(traducir_error_ytdlp(&texto_errores))),
        Err(e) => Err(Fallo::Mensaje(format!("yt-dlp falló: {e}"))),
    }
}

/// Cierra un proceso y todos sus subprocesos.
fn matar_arbol(pid: u32) {
    let mut comando = std::process::Command::new("taskkill");
    comando.args(["/PID", &pid.to_string(), "/T", "/F"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        comando.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = comando.output();
}

/// Al cerrar la app, detiene las descargas en curso para no dejar procesos
/// sueltos.
pub fn detener_todo(app: &AppHandle) {
    let cola = app.state::<Cola>();
    let pids: Vec<u32> = cola.0.lock().unwrap().entradas.iter().filter_map(|e| e.pid).collect();
    for pid in pids {
        matar_arbol(pid);
    }
}

#[tauri::command]
pub fn obtener_cola(cola: State<'_, Cola>) -> Vec<Descarga> {
    cola.0.lock().unwrap().lista()
}

/// Agrega videos a la cola y devuelve cuántos se agregaron (se saltan los que
/// ya están esperando o descargándose).
#[tauri::command]
pub fn agregar_a_cola(
    app: AppHandle,
    cola: State<'_, Cola>,
    videos: Vec<NuevaDescarga>,
    opciones: Opciones,
) -> usize {
    let agregados = {
        let mut cola = cola.0.lock().unwrap();
        let mut agregados = 0;
        for video in videos {
            let repetido = cola.entradas.iter().any(|e| {
                e.descarga.video_id == video.video_id
                    && (e.descarga.estado == Estado::EnEspera || e.descarga.estado.activo())
            });
            if repetido {
                continue;
            }
            let id = cola.siguiente_id;
            cola.siguiente_id += 1;
            cola.entradas.push(Entrada {
                descarga: Descarga {
                    id,
                    video_id: video.video_id,
                    titulo: video.titulo,
                    url: video.url,
                    miniatura: video.miniatura,
                    estado: Estado::EnEspera,
                    porcentaje: None,
                    velocidad: None,
                    eta: None,
                    parte: 0,
                    partes: 0,
                    proceso: None,
                    archivo: None,
                    nota: None,
                    error: None,
                },
                opciones: opciones.clone(),
                forzar: video.forzar,
                cancelar: None,
                pid: None,
            });
            agregados += 1;
        }
        avisar_cola(&app, &cola);
        agregados
    };
    programar(&app);
    agregados
}

#[tauri::command]
pub fn cancelar_descarga(app: AppHandle, cola: State<'_, Cola>, id: u64) {
    let mut cola = cola.0.lock().unwrap();
    let Some(entrada) = cola.entrada(id) else { return };
    match entrada.descarga.estado {
        Estado::EnEspera => {
            entrada.descarga.estado = Estado::Cancelado;
            let _ = app.emit(EVENTO_DESCARGA, entrada.descarga.clone());
        }
        estado if estado.activo() => {
            if let Some(cancelar) = &entrada.cancelar {
                cancelar.notify_one();
            }
        }
        _ => {}
    }
}

#[tauri::command]
pub fn reintentar_descarga(app: AppHandle, cola: State<'_, Cola>, id: u64) {
    {
        let mut cola = cola.0.lock().unwrap();
        let Some(entrada) = cola.entrada(id) else { return };
        if !matches!(entrada.descarga.estado, Estado::Error | Estado::Cancelado) {
            return;
        }
        let d = &mut entrada.descarga;
        d.estado = Estado::EnEspera;
        d.porcentaje = None;
        d.parte = 0;
        d.partes = 0;
        d.error = None;
        d.nota = None;
        let _ = app.emit(EVENTO_DESCARGA, d.clone());
    }
    programar(&app);
}

#[tauri::command]
pub fn quitar_de_cola(app: AppHandle, cola: State<'_, Cola>, id: u64) {
    let mut cola = cola.0.lock().unwrap();
    cola.entradas.retain(|e| e.descarga.id != id || e.descarga.estado.activo());
    avisar_cola(&app, &cola);
}

#[tauri::command]
pub fn limpiar_terminados(app: AppHandle, cola: State<'_, Cola>) {
    let mut cola = cola.0.lock().unwrap();
    cola.entradas.retain(|e| e.descarga.estado != Estado::Terminado);
    avisar_cola(&app, &cola);
}

#[tauri::command]
pub fn fijar_simultaneas(app: AppHandle, cola: State<'_, Cola>, cantidad: usize) {
    cola.0.lock().unwrap().maximo = cantidad.clamp(1, 4);
    programar(&app);
}
