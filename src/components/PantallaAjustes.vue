<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { appLocalDataDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import BarraProgreso from "./BarraProgreso.vue";
import { ajustes, CALIDADES, MAX_SIMULTANEAS, PLANTILLAS } from "../ajustes";
import {
  actualizarFfmpeg,
  actualizarYtdlp,
  estadoHerramientas,
  type EstadoHerramientas,
  type Progreso,
} from "../herramientas";

const props = defineProps<{ estado: EstadoHerramientas; progreso: Progreso | null }>();
const emit = defineEmits<{ estado: [estado: EstadoHerramientas] }>();

const actualizando = ref<"ytdlp" | "ffmpeg" | null>(null);
const avisoHerramientas = ref<string | null>(null);
const errorCarpeta = ref<string | null>(null);
const version = ref("");

onMounted(async () => {
  version.value = await getVersion();
});

async function elegirCarpeta() {
  const elegida = await open({
    directory: true,
    defaultPath: ajustes.carpetaDestino,
    title: "Elige dónde guardar los respaldos",
  });
  if (typeof elegida === "string") ajustes.carpetaDestino = elegida;
}

async function abrirCarpeta() {
  errorCarpeta.value = null;
  try {
    await invoke("abrir_carpeta", { ruta: ajustes.carpetaDestino });
  } catch (e) {
    errorCarpeta.value = String(e);
  }
}

async function abrirCarpetaHerramientas() {
  try {
    await invoke("abrir_carpeta", { ruta: await join(await appLocalDataDir(), "bin") });
  } catch (e) {
    avisoHerramientas.value = String(e);
  }
}

async function buscarActualizacionYtdlp() {
  actualizando.value = "ytdlp";
  try {
    const nueva = await actualizarYtdlp();
    emit("estado", await estadoHerramientas());
    avisoHerramientas.value = nueva
      ? `yt-dlp se actualizó a la versión ${nueva}.`
      : "yt-dlp ya está en la última versión.";
  } catch (e) {
    avisoHerramientas.value = String(e);
  } finally {
    actualizando.value = null;
  }
}

async function actualizarFfmpegAhora() {
  actualizando.value = "ffmpeg";
  try {
    const nuevo = await actualizarFfmpeg();
    emit("estado", nuevo);
    avisoHerramientas.value = `ffmpeg se actualizó (${nuevo.ffmpeg}).`;
  } catch (e) {
    avisoHerramientas.value = String(e);
  } finally {
    actualizando.value = null;
  }
}
</script>

<template>
  <section class="tarjeta">
    <h2>Carpeta de destino</h2>
    <div class="fila">
      <code class="ruta">{{ ajustes.carpetaDestino }}</code>
      <button @click="elegirCarpeta">Cambiar…</button>
      <button class="secundario" @click="abrirCarpeta">Abrir</button>
    </div>
    <p v-if="errorCarpeta" class="ayuda">{{ errorCarpeta }}</p>
  </section>

  <section class="tarjeta">
    <h2>Descargas</h2>

    <label class="campo">
      <span>Calidad</span>
      <select v-model="ajustes.calidad">
        <option v-for="c in CALIDADES" :key="c.valor" :value="c.valor">{{ c.texto }}</option>
      </select>
    </label>

    <label class="campo">
      <span>Descargas al mismo tiempo</span>
      <select v-model.number="ajustes.descargasSimultaneas">
        <option v-for="n in MAX_SIMULTANEAS" :key="n" :value="n">{{ n }}</option>
      </select>
    </label>

    <fieldset class="campo">
      <legend>Nombre de los archivos</legend>
      <label v-for="p in PLANTILLAS" :key="p.valor" class="opcion">
        <input v-model="ajustes.plantilla" type="radio" :value="p.valor" />
        <span>
          {{ p.texto }}
          <small>{{ p.ejemplo }}</small>
        </span>
      </label>
    </fieldset>
  </section>

  <section class="tarjeta">
    <h2>Herramientas</h2>
    <BarraProgreso
      nombre="yt-dlp"
      :version="props.estado.ytdlp"
      :progreso="actualizando === 'ytdlp' ? props.progreso : null"
    />
    <BarraProgreso
      nombre="ffmpeg"
      :version="props.estado.ffmpeg"
      :progreso="actualizando === 'ffmpeg' ? props.progreso : null"
    />
    <div class="fila">
      <button :disabled="actualizando !== null" @click="buscarActualizacionYtdlp">
        Buscar actualización de yt-dlp
      </button>
      <button :disabled="actualizando !== null" @click="actualizarFfmpegAhora">
        Actualizar ffmpeg
      </button>
    </div>
    <p v-if="avisoHerramientas" class="ayuda">{{ avisoHerramientas }}</p>
  </section>

  <section class="tarjeta acerca">
    <h2>Acerca de</h2>
    <p><strong>Respaldo de Videos</strong> {{ version && `versión ${version}` }}</p>
    <p>
      Te ayuda a guardar una copia de tus propios videos. Úsala solo con videos que te pertenecen
      o que tienes permiso de descargar.
    </p>
    <p class="suave">
      Funciona gracias a
      <button class="enlace" @click="openUrl('https://github.com/yt-dlp/yt-dlp')">yt-dlp</button>
      y
      <button class="enlace" @click="openUrl('https://ffmpeg.org')">FFmpeg</button>, que se
      descargan desde sus sitios oficiales y se usan sin cambios.
    </p>
    <button class="secundario" @click="abrirCarpetaHerramientas">
      Abrir carpeta de herramientas
    </button>
  </section>
</template>

<style scoped>
.fila {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.75rem;
}

.ruta {
  flex: 1;
  min-width: 0;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  background: var(--fondo);
  overflow-wrap: anywhere;
}

.campo {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  margin: 0 0 1.25rem;
  padding: 0;
  border: none;
}

.campo > span,
.campo > legend {
  font-weight: 600;
  margin-bottom: 0.35rem;
}

select {
  font: inherit;
  max-width: 340px;
  padding: 0.4rem 0.5rem;
  border: 1px solid var(--borde);
  border-radius: 6px;
  background: var(--superficie);
  color: var(--texto);
}

.opcion {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  cursor: pointer;
}

.opcion input {
  margin-top: 0.35rem;
}

.opcion small {
  display: block;
  color: var(--texto-suave);
}

.ayuda {
  margin: 0.75rem 0 0;
  color: var(--texto-suave);
}
</style>
