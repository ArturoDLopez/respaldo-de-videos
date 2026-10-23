<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import PantallaPreparacion from "./components/PantallaPreparacion.vue";
import BarraProgreso from "./components/BarraProgreso.vue";
import {
  actualizarFfmpeg,
  actualizarYtdlp,
  escucharProgreso,
  estadoHerramientas,
  type EstadoHerramientas,
  type Progreso,
} from "./herramientas";

const estado = ref<EstadoHerramientas | null>(null);
const listo = ref(false);
const aviso = ref<string | null>(null);
const actualizando = ref<"ytdlp" | "ffmpeg" | null>(null);
const progreso = ref<Progreso | null>(null);
let dejarDeEscuchar: UnlistenFn | undefined;

async function buscarActualizacionYtdlp(silencioso: boolean) {
  actualizando.value = "ytdlp";
  progreso.value = null;
  try {
    const nueva = await actualizarYtdlp();
    estado.value = await estadoHerramientas();
    if (nueva) aviso.value = `yt-dlp se actualizó a la versión ${nueva}.`;
    else if (!silencioso) aviso.value = "yt-dlp ya está en la última versión.";
  } catch (e) {
    // Al iniciar no molestamos si no hay internet; se usa la versión instalada.
    if (!silencioso) aviso.value = String(e);
  } finally {
    actualizando.value = null;
  }
}

async function actualizarFfmpegAhora() {
  actualizando.value = "ffmpeg";
  progreso.value = null;
  try {
    estado.value = await actualizarFfmpeg();
    aviso.value = `ffmpeg se actualizó (${estado.value.ffmpeg}).`;
  } catch (e) {
    aviso.value = String(e);
  } finally {
    actualizando.value = null;
  }
}

function alTerminarPreparacion(nuevo: EstadoHerramientas) {
  estado.value = nuevo;
  listo.value = true;
}

onMounted(async () => {
  dejarDeEscuchar = await escucharProgreso((p) => (progreso.value = p));
  estado.value = await estadoHerramientas();
  if (estado.value.ytdlp && estado.value.ffmpeg) {
    listo.value = true;
    buscarActualizacionYtdlp(true);
  }
});

onUnmounted(() => dejarDeEscuchar?.());
</script>

<template>
  <PantallaPreparacion v-if="estado && !listo" :estado="estado" @listo="alTerminarPreparacion" />

  <main v-else-if="estado" class="contenedor">
    <h1>Respaldo de Videos</h1>
    <p>Respalda tus propios videos con yt-dlp y ffmpeg.</p>

    <section class="tarjeta">
      <h2>Herramientas</h2>
      <BarraProgreso
        nombre="yt-dlp"
        :version="estado.ytdlp"
        :progreso="actualizando === 'ytdlp' ? progreso : null"
      />
      <BarraProgreso
        nombre="ffmpeg"
        :version="estado.ffmpeg"
        :progreso="actualizando === 'ffmpeg' ? progreso : null"
      />
      <div class="botones">
        <button :disabled="actualizando !== null" @click="buscarActualizacionYtdlp(false)">
          Buscar actualización de yt-dlp
        </button>
        <button :disabled="actualizando !== null" @click="actualizarFfmpegAhora">
          Actualizar ffmpeg
        </button>
      </div>
      <p v-if="aviso" class="aviso">{{ aviso }}</p>
    </section>
  </main>
</template>

<style>
:root {
  --texto: #1a1a1a;
  --texto-suave: #5c5c5c;
  --fondo: #f6f6f6;
  --superficie: #ffffff;
  --borde: #dddddd;
  --acento: #2f6fdb;
  --acento-hover: #2559b3;
  --error-fondo: #fde8e8;
  --error-texto: #9b1c1c;

  font-family: "Segoe UI", system-ui, sans-serif;
  font-size: 16px;
  line-height: 1.5;
  color: var(--texto);
  background-color: var(--fondo);
}

@media (prefers-color-scheme: dark) {
  :root {
    --texto: #f0f0f0;
    --texto-suave: #a8a8a8;
    --fondo: #1e1e1e;
    --superficie: #2a2a2a;
    --borde: #3d3d3d;
    --acento: #5b93f0;
    --acento-hover: #7aa8f3;
    --error-fondo: #3b1d1d;
    --error-texto: #f5b4b4;
  }
}

body {
  margin: 0;
}

button {
  font: inherit;
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 6px;
  background: var(--acento);
  color: #fff;
  cursor: pointer;
}

button:hover:not(:disabled) {
  background: var(--acento-hover);
}

button:disabled {
  opacity: 0.5;
  cursor: default;
}

.contenedor {
  max-width: 760px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
}

.tarjeta {
  margin-top: 1.5rem;
  padding: 1.25rem 1.5rem;
  border: 1px solid var(--borde);
  border-radius: 10px;
  background: var(--superficie);
}

.tarjeta h2 {
  margin-top: 0;
  font-size: 1.15rem;
}

.botones {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.aviso {
  margin: 1rem 0 0;
  color: var(--texto-suave);
}
</style>
