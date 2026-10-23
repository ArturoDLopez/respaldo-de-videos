import { reactive } from "vue";
import { load, type Store } from "@tauri-apps/plugin-store";

export interface EntradaHistorial {
  videoId: string;
  titulo: string;
  url: string;
  miniatura: string | null;
  /** Ruta del archivo descargado. */
  archivo: string;
  /** Milisegundos desde 1970. */
  fecha: number;
}

/** Más recientes primero. */
export const historial = reactive<EntradaHistorial[]>([]);

const MAXIMO = 5000;
let almacen: Store | undefined;

export async function cargarHistorial() {
  almacen = await load("historial.json", { autoSave: 500, defaults: {} });
  historial.splice(0, historial.length, ...((await almacen.get<EntradaHistorial[]>("entradas")) ?? []));
}

function guardar() {
  almacen?.set("entradas", [...historial]);
}

export function registrarEnHistorial(entrada: EntradaHistorial) {
  historial.unshift(entrada);
  if (historial.length > MAXIMO) historial.splice(MAXIMO);
  guardar();
}

/** Borra solo la lista; los archivos descargados no se tocan. */
export function borrarHistorial() {
  historial.splice(0);
  guardar();
}
