<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import BarraProgreso from "./BarraProgreso.vue";
import {
  escucharProgreso,
  instalarHerramientas,
  NOMBRES,
  type EstadoHerramientas,
  type Herramienta,
  type Progreso,
} from "../herramientas";

const props = defineProps<{ estado: EstadoHerramientas }>();
const emit = defineEmits<{ listo: [estado: EstadoHerramientas] }>();

const progresos = reactive<Record<Herramienta, Progreso | null>>({ ytdlp: null, ffmpeg: null });
const error = ref<string | null>(null);
const trabajando = ref(false);
let dejarDeEscuchar: UnlistenFn | undefined;

async function instalar() {
  error.value = null;
  trabajando.value = true;
  try {
    emit("listo", await instalarHerramientas());
  } catch (e) {
    error.value = String(e);
  } finally {
    trabajando.value = false;
  }
}

onMounted(async () => {
  dejarDeEscuchar = await escucharProgreso((p) => (progresos[p.herramienta] = p));
  instalar();
});

onUnmounted(() => dejarDeEscuchar?.());
</script>

<template>
  <main class="preparacion">
    <h1>Preparando herramientas…</h1>
    <p class="explicacion">
      La primera vez que se abre, la app descarga yt-dlp y ffmpeg (unos 200 MB). Solo tarda un
      momento.
    </p>

    <BarraProgreso
      v-for="h in (['ytdlp', 'ffmpeg'] as const)"
      :key="h"
      :nombre="NOMBRES[h]"
      :progreso="progresos[h]"
      :version="props.estado[h]"
    />

    <div v-if="error" class="error">
      <p>{{ error }}</p>
      <button :disabled="trabajando" @click="instalar">Reintentar</button>
    </div>
  </main>
</template>

<style scoped>
.preparacion {
  max-width: 520px;
  margin: 12vh auto 0;
  padding: 0 1.5rem;
}

.explicacion {
  color: var(--texto-suave);
  margin-bottom: 2rem;
}

.error {
  margin-top: 1.5rem;
  padding: 1rem;
  border-radius: 8px;
  background: var(--error-fondo);
  color: var(--error-texto);
}

.error p {
  margin: 0 0 0.75rem;
}
</style>
