<script setup lang="ts">
import { computed, ref } from "vue";
import ListaVideos from "./ListaVideos.vue";
import { ajustes } from "../ajustes";
import {
  analizarEnlace,
  cancelarAnalisis,
  TIPOS,
  type Analisis,
} from "../analisis";

const emit = defineEmits<{ irAjustes: [] }>();

const enlace = ref("");
const analizando = ref(false);
const error = ref<string | null>(null);
const resultado = ref<Analisis | null>(null);
const seleccion = ref(new Set<string>());

const resumen = computed(() => {
  if (!resultado.value) return "";
  const elementos = resultado.value.elementos;
  const total = elementos.length;
  const respaldados = elementos.filter((e) => e.estado === "respaldado").length;
  const partes = [total === 1 ? "1 video" : `${total} videos`];
  if (respaldados === 1) partes.push("1 ya respaldado");
  else if (respaldados > 1) partes.push(`${respaldados} ya respaldados`);
  return partes.join(" · ");
});

async function analizar() {
  if (!enlace.value.trim() || analizando.value) return;
  analizando.value = true;
  error.value = null;
  resultado.value = null;
  try {
    const analisis = await analizarEnlace(enlace.value, ajustes.carpetaDestino);
    // Por defecto se marcan solo los que faltan por respaldar.
    seleccion.value = new Set(
      analisis.elementos.filter((e) => e.estado === "disponible").map((e) => e.id),
    );
    resultado.value = analisis;
    if (analisis.elementos.length === 0) error.value = "El enlace no tiene videos para descargar.";
  } catch (e) {
    error.value = String(e);
  } finally {
    analizando.value = false;
  }
}
</script>

<template>
  <section class="tarjeta">
    <form class="buscador" @submit.prevent="analizar">
      <input
        v-model="enlace"
        type="text"
        placeholder="Pega el enlace de un video, una playlist o un canal"
        :disabled="analizando"
        spellcheck="false"
        autofocus
      />
      <button v-if="!analizando" type="submit" :disabled="!enlace.trim()">Analizar</button>
      <button v-else type="button" class="secundario" @click="cancelarAnalisis">Cancelar</button>
    </form>
    <p class="destino">
      Se guardará en <code>{{ ajustes.carpetaDestino }}</code>
      <button class="enlace" @click="emit('irAjustes')">Cambiar</button>
    </p>

    <p v-if="analizando" class="suave">
      Analizando… En canales con muchos videos esto puede tardar un par de minutos.
    </p>
    <p v-if="error" class="error">{{ error }}</p>
  </section>

  <section v-if="resultado && resultado.elementos.length" class="tarjeta">
    <header class="encabezado">
      <div>
        <span class="tipo">{{ TIPOS[resultado.tipo] }}</span>
        <h2>{{ resultado.titulo }}</h2>
        <p class="suave">{{ resumen }}</p>
      </div>
    </header>

    <ListaVideos v-model="seleccion" :elementos="resultado.elementos" />

    <footer class="pie">
      <span>{{ seleccion.size }} seleccionados</span>
      <button :disabled="true" title="La cola de descargas llega en la Fase 4">
        Agregar a la cola
      </button>
    </footer>
  </section>
</template>

<style scoped>
.buscador {
  display: flex;
  gap: 0.75rem;
}

.buscador input {
  flex: 1;
  min-width: 0;
  font: inherit;
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--borde);
  border-radius: 6px;
  background: var(--fondo);
  color: var(--texto);
}

.buscador input:focus {
  outline: 2px solid var(--acento);
  outline-offset: -1px;
}

.destino {
  margin: 0.75rem 0 0;
  font-size: 0.9rem;
  color: var(--texto-suave);
}

.error {
  margin: 1rem 0 0;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  background: var(--error-fondo);
  color: var(--error-texto);
}

.suave {
  margin: 1rem 0 0;
}

.encabezado h2 {
  margin: 0.15rem 0 0;
}

.encabezado .suave {
  margin: 0.15rem 0 1rem;
}

.tipo {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--acento);
}

.pie {
  position: sticky;
  bottom: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: 0 -1.5rem -1.25rem;
  padding: 0.75rem 1.5rem;
  border-top: 1px solid var(--borde);
  border-radius: 0 0 10px 10px;
  background: var(--superficie);
}
</style>
