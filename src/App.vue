<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import PantallaPreparacion from "./components/PantallaPreparacion.vue";
import PantallaAjustes from "./components/PantallaAjustes.vue";
import PantallaDescargar from "./components/PantallaDescargar.vue";
import { cargarAjustes } from "./ajustes";
import {
  actualizarYtdlp,
  escucharProgreso,
  estadoHerramientas,
  type EstadoHerramientas,
  type Progreso,
} from "./herramientas";

type Pestana = "descargar" | "ajustes";

const PESTANAS: { valor: Pestana; texto: string }[] = [
  { valor: "descargar", texto: "Descargar" },
  { valor: "ajustes", texto: "Ajustes" },
];

const estado = ref<EstadoHerramientas | null>(null);
const listo = ref(false);
const pestana = ref<Pestana>("descargar");
const aviso = ref<string | null>(null);
const progreso = ref<Progreso | null>(null);
let dejarDeEscuchar: UnlistenFn | undefined;

/** Al iniciar se busca una versión nueva de yt-dlp sin molestar si falla. */
async function actualizarYtdlpEnSilencio() {
  try {
    const nueva = await actualizarYtdlp();
    if (nueva) {
      estado.value = await estadoHerramientas();
      aviso.value = `yt-dlp se actualizó a la versión ${nueva}.`;
    }
  } catch {
    // Sin internet se sigue usando la versión instalada.
  }
}

function alTerminarPreparacion(nuevo: EstadoHerramientas) {
  estado.value = nuevo;
  listo.value = true;
}

onMounted(async () => {
  dejarDeEscuchar = await escucharProgreso((p) => (progreso.value = p));
  await cargarAjustes();
  estado.value = await estadoHerramientas();
  if (estado.value.ytdlp && estado.value.ffmpeg) {
    listo.value = true;
    actualizarYtdlpEnSilencio();
  }
});

onUnmounted(() => dejarDeEscuchar?.());
</script>

<template>
  <PantallaPreparacion v-if="estado && !listo" :estado="estado" @listo="alTerminarPreparacion" />

  <template v-else-if="estado">
    <nav class="pestanas">
      <span class="marca">Respaldo de Videos</span>
      <button
        v-for="p in PESTANAS"
        :key="p.valor"
        class="pestana"
        :class="{ activa: pestana === p.valor }"
        @click="pestana = p.valor"
      >
        {{ p.texto }}
      </button>
    </nav>

    <div v-if="aviso" class="aviso">
      <span>{{ aviso }}</span>
      <button class="cerrar" title="Cerrar" @click="aviso = null">×</button>
    </div>

    <main class="contenedor">
      <!-- v-show conserva el análisis al cambiar de pestaña. -->
      <PantallaDescargar v-show="pestana === 'descargar'" @ir-ajustes="pestana = 'ajustes'" />

      <PantallaAjustes
        v-if="pestana === 'ajustes'"
        :estado="estado"
        :progreso="progreso"
        @estado="(nuevo) => (estado = nuevo)"
      />
    </main>
  </template>
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
  --aviso-fondo: #e6f0ff;
  --exito: #1e7a3c;

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
    --aviso-fondo: #1f2d44;
    --exito: #6fd08f;
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

button.secundario {
  background: transparent;
  color: var(--texto);
  border: 1px solid var(--borde);
}

button.secundario:hover:not(:disabled) {
  background: var(--fondo);
}

button.enlace {
  padding: 0 0.25rem;
  background: none;
  color: var(--acento);
  text-decoration: underline;
}

button.enlace:hover:not(:disabled) {
  background: none;
  color: var(--acento-hover);
}

code {
  font-family: Consolas, monospace;
  font-size: 0.9em;
}

.contenedor {
  max-width: 760px;
  margin: 0 auto;
  padding: 1.5rem;
}

.tarjeta {
  margin-bottom: 1.25rem;
  padding: 1.25rem 1.5rem;
  border: 1px solid var(--borde);
  border-radius: 10px;
  background: var(--superficie);
}

.tarjeta h2 {
  margin-top: 0;
  font-size: 1.15rem;
}

.suave {
  color: var(--texto-suave);
}

.pestanas {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0 1.5rem;
  border-bottom: 1px solid var(--borde);
  background: var(--superficie);
}

.marca {
  font-weight: 700;
  margin-right: 1.5rem;
}

.pestana {
  padding: 0.85rem 1rem;
  border-radius: 0;
  border-bottom: 2px solid transparent;
  background: none;
  color: var(--texto-suave);
}

.pestana:hover:not(:disabled) {
  background: none;
  color: var(--texto);
}

.pestana.activa {
  color: var(--acento);
  border-bottom-color: var(--acento);
}

.aviso {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 1.5rem;
  background: var(--aviso-fondo);
}

.cerrar {
  padding: 0 0.5rem;
  background: none;
  color: var(--texto);
  font-size: 1.25rem;
}

.cerrar:hover:not(:disabled) {
  background: none;
}
</style>
