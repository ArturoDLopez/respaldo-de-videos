import { computed, reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Ajustes } from "./ajustes";
import type { Elemento } from "./analisis";

export type EstadoDescarga =
  | "en_espera"
  | "descargando"
  | "procesando"
  | "terminado"
  | "error"
  | "cancelado";

export interface Descarga {
  id: number;
  videoId: string;
  titulo: string;
  url: string;
  miniatura: string | null;
  estado: EstadoDescarga;
  /** De 0 a 100 dentro de la parte actual. */
  porcentaje: number | null;
  /** Bytes por segundo. */
  velocidad: number | null;
  /** Segundos restantes. */
  eta: number | null;
  parte: number;
  partes: number;
  /** Paso de ffmpeg en curso ("Merger", "EmbedThumbnail"…). */
  proceso: string | null;
  archivo: string | null;
  nota: string | null;
  error: string | null;
}

/** La cola tal como la tiene Rust; se mantiene al día con sus eventos. */
export const cola = reactive<Descarga[]>([]);

export const activas = computed(
  () => cola.filter((d) => d.estado === "descargando" || d.estado === "procesando").length,
);
export const pendientes = computed(
  () => cola.filter((d) => d.estado === "en_espera").length + activas.value,
);

export async function iniciarCola() {
  await listen<Descarga[]>("cola-actualizada", (e) => cola.splice(0, cola.length, ...e.payload));
  await listen<Descarga>("descarga-actualizada", (e) => {
    const i = cola.findIndex((d) => d.id === e.payload.id);
    if (i >= 0) cola[i] = e.payload;
  });
  cola.splice(0, cola.length, ...(await invoke<Descarga[]>("obtener_cola")));
}

/** Devuelve cuántos se agregaron (se saltan los que ya están en la cola). */
export function agregarACola(elementos: Elemento[], ajustes: Ajustes) {
  return invoke<number>("agregar_a_cola", {
    videos: elementos.map((e) => ({
      videoId: e.id,
      titulo: e.titulo,
      url: e.url,
      miniatura: e.miniatura,
      forzar: e.estado === "respaldado",
    })),
    opciones: {
      carpeta: ajustes.carpetaDestino,
      calidad: ajustes.calidad,
      plantilla: ajustes.plantilla,
    },
  });
}

export const cancelarDescarga = (id: number) => invoke<void>("cancelar_descarga", { id });
export const reintentarDescarga = (id: number) => invoke<void>("reintentar_descarga", { id });
export const quitarDeCola = (id: number) => invoke<void>("quitar_de_cola", { id });
export const limpiarTerminados = () => invoke<void>("limpiar_terminados");
export const fijarSimultaneas = (cantidad: number) =>
  invoke<void>("fijar_simultaneas", { cantidad });

const PROCESOS: Record<string, string> = {
  Merger: "Uniendo video y audio",
  ExtractAudio: "Extrayendo audio",
  Metadata: "Agregando información",
  EmbedThumbnail: "Agregando miniatura",
  MoveFiles: "Moviendo a la carpeta",
};

function formatoTamano(bytesPorSegundo: number) {
  const mb = bytesPorSegundo / 1024 / 1024;
  return mb >= 1 ? `${mb.toFixed(1)} MB/s` : `${(bytesPorSegundo / 1024).toFixed(0)} KB/s`;
}

function formatoTiempo(segundos: number) {
  const s = Math.round(segundos);
  const m = Math.floor(s / 60);
  return m >= 60
    ? `${Math.floor(m / 60)} h ${m % 60} min`
    : `${m}:${String(s % 60).padStart(2, "0")}`;
}

export function textoEstado(d: Descarga): string {
  switch (d.estado) {
    case "en_espera":
      return "En espera";
    case "descargando": {
      const partes = [];
      if (d.partes > 1) partes.push(d.parte === 1 ? "Video" : d.parte === 2 ? "Audio" : `Parte ${d.parte}`);
      partes.push(d.porcentaje === null ? "Descargando…" : `${d.porcentaje.toFixed(0)}%`);
      if (d.velocidad) partes.push(formatoTamano(d.velocidad));
      if (d.eta) partes.push(`faltan ${formatoTiempo(d.eta)}`);
      return partes.join(" · ");
    }
    case "procesando":
      return `${PROCESOS[d.proceso ?? ""] ?? "Procesando"}…`;
    case "terminado":
      return d.nota ?? "Terminado";
    case "error":
      return d.error ?? "Error";
    case "cancelado":
      return "Cancelado";
  }
}
