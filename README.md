# Respaldo de Videos

App de escritorio para Windows que te ayuda a guardar una copia de **tus propios videos** de YouTube en tu computadora. Pega el enlace de un video, una playlist o tu canal completo, elige cuáles quieres y la app los descarga en la mejor calidad disponible.

## Qué hace

- **Descarga videos, playlists o canales completos**: pega un enlace o escribe `@tucanal`.
- **Solo lo nuevo**: recuerda qué videos ya respaldaste en cada carpeta y los salta.
- **Cola de descargas** con progreso, velocidad y tiempo restante; puedes cancelar, reintentar y elegir cuántas descargas van al mismo tiempo.
- **Búsqueda de videos** sin necesidad de cuentas ni claves de API.
- **Calidad a elegir**: la mejor en MP4, 1080p, 720p o solo el audio.
- **Archivos ordenados** por canal y fecha, con miniatura, capítulos y datos del video incluidos.
- **Historial** de todo lo descargado.
- Toda la interfaz está en español.

La primera vez que la abras, descarga sola las herramientas que necesita ([yt-dlp](https://github.com/yt-dlp/yt-dlp) y [FFmpeg](https://ffmpeg.org)) y mantiene yt-dlp actualizado.

## Descargar e instalar

1. Ve a [Releases](https://github.com/ArturoDLopez/respaldo-de-videos/releases/latest) y descarga el archivo que termina en `x64-setup.exe`.
2. Ábrelo y sigue los pasos del instalador.

**Requisitos:** Windows 10 u 11 de 64 bits y unos 400 MB libres para las herramientas.

### Si Windows muestra un aviso

El instalador todavía no tiene firma digital (ver [Política de firma de código](#política-de-firma-de-código-code-signing-policy)), así que Windows puede desconfiar de él:

- **"Windows protegió su PC"** (SmartScreen): haz clic en **Más información** y luego en **Ejecutar de todas formas**.
- **"Control inteligente de aplicaciones bloqueó..."**: esta protección de Windows 11 no permite abrir programas sin firma. Por ahora, la única forma de instalarla es desactivarla en *Seguridad de Windows → Control de aplicaciones y navegador → Control inteligente de aplicaciones*. Esto dejará de ser necesario cuando el instalador esté firmado.

## Uso responsable

Esta app es para respaldar videos **que te pertenecen** o que tienes permiso de descargar. Descargar contenido de otras personas sin autorización puede violar sus derechos de autor y los términos de servicio de YouTube. Tú eres responsable de cómo la usas.

## Privacidad

Respaldo de Videos no recopila ni envía información tuya a nadie. No tiene anuncios, cuentas ni estadísticas de uso. Solo se conecta a internet para:

- **GitHub** (`github.com`): descargar y actualizar yt-dlp y FFmpeg desde sus publicaciones oficiales.
- **YouTube** (`youtube.com`, `googlevideo.com`, `ytimg.com`): analizar los enlaces y búsquedas que tú escribes, mostrar las miniaturas y descargar los videos que eliges.

Tus ajustes y tu historial se guardan solo en tu computadora, en `%APPDATA%\com.arturo.respaldodevideos`. Las herramientas se guardan en `%LOCALAPPDATA%\com.arturo.respaldodevideos\bin`.

## Política de firma de código (Code signing policy)

Los instaladores se compilan automáticamente con [GitHub Actions](.github/workflows/publicar.yml) a partir del código de este repositorio.

Se solicitó firma gratuita para proyectos de código abierto a [SignPath Foundation](https://signpath.org). Mientras no esté aprobada, los instaladores se publican sin firmar.

Equipo:

- **Autor, revisor y aprobador:** [ArturoDLopez](https://github.com/ArturoDLopez)

Privacidad: consulta la sección [Privacidad](#privacidad). Este programa no transfiere información a otros sistemas de la red, salvo cuando el usuario lo pide.

## Desarrollo

Hecha con [Tauri 2](https://tauri.app), Vue 3 y TypeScript. Necesitas Node.js, Rust y las Build Tools de Visual Studio con C++.

```
npm install
npm run tauri dev
```

Para generar el instalador en tu computadora:

```
npm run tauri build
```

### Publicar una versión

1. Cambia la versión en `package.json`, `src-tauri/Cargo.toml` y `src-tauri/tauri.conf.json`.
2. Crea y sube la etiqueta: `git tag v0.2.0` y `git push origin v0.2.0`.
3. GitHub Actions compila el instalador y crea un borrador en [Releases](https://github.com/ArturoDLopez/respaldo-de-videos/releases). Revísalo y publícalo.

## Licencia

MIT. Consulta el archivo [LICENSE](LICENSE).

yt-dlp y FFmpeg no se incluyen en este repositorio ni en el instalador: la app los descarga de sus sitios oficiales y cada uno tiene su propia licencia ([Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE) y [GPL](https://ffmpeg.org/legal.html), respectivamente).
