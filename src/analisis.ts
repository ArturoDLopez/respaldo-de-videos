import { invoke } from "@tauri-apps/api/core";

export type EstadoElemento = "disponible" | "respaldado" | "en_vivo" | "programado";

export interface Elemento {
  id: string;
  titulo: string;
  url: string;
  /** Pestaña del canal de donde viene ("Videos", "Shorts", "Live"…). */
  seccion: string | null;
  canal: string | null;
  vistas: number | null;
  duracion: number | null;
  miniatura: string | null;
  estado: EstadoElemento;
}

export interface Analisis {
  tipo: "video" | "playlist" | "canal";
  titulo: string;
  elementos: Elemento[];
}

export const TIPOS: Record<Analisis["tipo"], string> = {
  video: "Video",
  playlist: "Playlist",
  canal: "Canal",
};

export function analizarEnlace(enlace: string, carpeta: string) {
  return invoke<Analisis>("analizar_enlace", { enlace, carpeta });
}

export function buscarVideos(texto: string, cantidad: number, carpeta: string) {
  return invoke<Elemento[]>("buscar_videos", { texto, cantidad, carpeta });
}

export function cancelarConsulta(clave: "analisis" | "busqueda") {
  return invoke<void>("cancelar_consulta", { clave });
}

export function formatoVistas(vistas: number | null): string {
  if (vistas === null) return "";
  const texto = new Intl.NumberFormat("es", { notation: "compact" }).format(vistas);
  return vistas === 1 ? "1 vista" : `${texto} vistas`;
}

/** Solo se pueden descargar los que no están en vivo ni programados. */
export function sePuedeDescargar(e: Elemento) {
  return e.estado === "disponible" || e.estado === "respaldado";
}

export function formatoDuracion(segundos: number | null): string {
  if (segundos === null) return "";
  const s = Math.round(segundos);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const resto = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${resto}` : `${m}:${resto}`;
}
