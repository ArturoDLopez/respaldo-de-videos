import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Herramienta = "ytdlp" | "ffmpeg";

export interface EstadoHerramientas {
  /** Versión instalada, o null si falta. */
  ytdlp: string | null;
  ffmpeg: string | null;
}

export interface Progreso {
  herramienta: Herramienta;
  etapa: "descargando" | "verificando" | "extrayendo" | "listo";
  descargado: number;
  total: number | null;
}

export const NOMBRES: Record<Herramienta, string> = {
  ytdlp: "yt-dlp",
  ffmpeg: "ffmpeg",
};

export function estadoHerramientas() {
  return invoke<EstadoHerramientas>("estado_herramientas");
}

export function instalarHerramientas() {
  return invoke<EstadoHerramientas>("instalar_herramientas");
}

/** Devuelve la versión nueva, o null si yt-dlp ya estaba al día. */
export function actualizarYtdlp() {
  return invoke<string | null>("actualizar_ytdlp");
}

export function actualizarFfmpeg() {
  return invoke<EstadoHerramientas>("actualizar_ffmpeg");
}

export function escucharProgreso(alAvisar: (p: Progreso) => void): Promise<UnlistenFn> {
  return listen<Progreso>("herramientas-progreso", (e) => alAvisar(e.payload));
}

export function textoProgreso(p: Progreso): string {
  switch (p.etapa) {
    case "descargando": {
      const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
      return p.total
        ? `Descargando… ${mb(p.descargado)} de ${mb(p.total)} MB`
        : `Descargando… ${mb(p.descargado)} MB`;
    }
    case "verificando":
      return "Verificando la descarga…";
    case "extrayendo":
      return "Extrayendo archivos…";
    case "listo":
      return "Listo";
  }
}

/** Porcentaje de 0 a 100, o null si no se puede calcular. */
export function porcentaje(p: Progreso): number | null {
  if (p.etapa === "listo") return 100;
  if (p.etapa !== "descargando" || !p.total) return null;
  return Math.min(100, (p.descargado / p.total) * 100);
}
