import { reactive, watch } from "vue";
import { load, type Store } from "@tauri-apps/plugin-store";
import { join, videoDir } from "@tauri-apps/api/path";

export type Calidad = "mejor" | "1080" | "720" | "audio";
export type Plantilla = "canal-fecha" | "fecha" | "titulo";

export interface Ajustes {
  carpetaDestino: string;
  calidad: Calidad;
  descargasSimultaneas: number;
  plantilla: Plantilla;
}

export const CALIDADES: { valor: Calidad; texto: string }[] = [
  { valor: "mejor", texto: "Mejor calidad disponible (MP4)" },
  { valor: "1080", texto: "Hasta 1080p (MP4)" },
  { valor: "720", texto: "Hasta 720p (MP4)" },
  { valor: "audio", texto: "Solo audio (M4A)" },
];

export const PLANTILLAS: { valor: Plantilla; texto: string; ejemplo: string }[] = [
  {
    valor: "canal-fecha",
    texto: "Carpeta por canal, con fecha",
    ejemplo: "Mi Canal\\2026-10-21 - Mi video [dQw4w9WgXcQ].mp4",
  },
  {
    valor: "fecha",
    texto: "Todo junto, con fecha",
    ejemplo: "2026-10-21 - Mi video [dQw4w9WgXcQ].mp4",
  },
  {
    valor: "titulo",
    texto: "Solo el título",
    ejemplo: "Mi video [dQw4w9WgXcQ].mp4",
  },
];

export const MAX_SIMULTANEAS = 4;

/** Los ajustes actuales. Se guardan solos en disco cada vez que cambian. */
export const ajustes = reactive<Ajustes>({
  carpetaDestino: "",
  calidad: "mejor",
  descargasSimultaneas: 2,
  plantilla: "canal-fecha",
});

let almacen: Store | undefined;

export async function cargarAjustes() {
  almacen = await load("ajustes.json", { autoSave: 300, defaults: {} });
  const guardados = await almacen.get<Partial<Ajustes>>("ajustes");
  Object.assign(ajustes, guardados ?? {});

  if (!ajustes.carpetaDestino) {
    ajustes.carpetaDestino = await join(await videoDir(), "Respaldo de Videos");
  }
  ajustes.descargasSimultaneas = Math.min(
    MAX_SIMULTANEAS,
    Math.max(1, Math.round(ajustes.descargasSimultaneas) || 1),
  );

  watch(ajustes, () => almacen?.set("ajustes", { ...ajustes }), { deep: true, immediate: true });
}
