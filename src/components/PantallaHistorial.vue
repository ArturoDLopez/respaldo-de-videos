<script setup lang="ts">
import { computed, ref } from "vue";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { borrarHistorial, historial, type EntradaHistorial } from "../historial";

const filtro = ref("");
const confirmarBorrado = ref(false);
const error = ref<string | null>(null);

const formatoFecha = new Intl.DateTimeFormat("es", { dateStyle: "medium", timeStyle: "short" });

const visibles = computed(() => {
  const texto = filtro.value.trim().toLowerCase();
  return texto ? historial.filter((h) => h.titulo.toLowerCase().includes(texto)) : historial;
});

async function mostrar(h: EntradaHistorial) {
  error.value = null;
  try {
    await revealItemInDir(h.archivo);
  } catch {
    error.value = `El archivo de "${h.titulo}" ya no está en ${h.archivo}. Puede que lo hayas movido o borrado.`;
  }
}

function borrar() {
  borrarHistorial();
  confirmarBorrado.value = false;
}
</script>

<template>
  <section class="tarjeta">
    <header class="encabezado">
      <div>
        <h2>Historial</h2>
        <p class="suave">
          {{ historial.length === 1 ? "1 video descargado" : `${historial.length} videos descargados` }}
        </p>
      </div>
      <div v-if="historial.length" class="borrar">
        <template v-if="confirmarBorrado">
          <span class="suave">¿Borrar la lista? Los archivos no se tocan.</span>
          <button class="peligro" @click="borrar">Sí, borrar</button>
          <button class="secundario" @click="confirmarBorrado = false">No</button>
        </template>
        <button v-else class="secundario" @click="confirmarBorrado = true">Borrar historial</button>
      </div>
    </header>

    <input
      v-if="historial.length"
      v-model="filtro"
      class="filtro"
      type="search"
      placeholder="Filtrar por título"
      spellcheck="false"
    />
    <p v-if="error" class="error">{{ error }}</p>

    <p v-if="!historial.length" class="suave">
      Aquí aparecerán los videos que descargues.
    </p>
    <p v-else-if="!visibles.length" class="suave">Ningún video coincide con el filtro.</p>

    <ul>
      <li v-for="h in visibles" :key="`${h.fecha}-${h.videoId}`">
        <span class="miniatura">
          <img v-if="h.miniatura" :src="h.miniatura" alt="" loading="lazy" />
        </span>
        <div class="datos">
          <span class="titulo">{{ h.titulo }}</span>
          <span class="fecha">{{ formatoFecha.format(h.fecha) }}</span>
        </div>
        <div class="botones">
          <button class="secundario" @click="mostrar(h)">Mostrar</button>
          <button class="secundario" @click="openUrl(h.url)">Ver en YouTube</button>
        </div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.encabezado {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
}

.encabezado .suave {
  margin: 0.15rem 0 0.75rem;
}

.borrar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.9rem;
}

.peligro {
  background: var(--error-texto);
}

.peligro:hover:not(:disabled) {
  background: var(--error-texto);
  opacity: 0.85;
}

.filtro {
  width: 100%;
  box-sizing: border-box;
  margin-bottom: 0.75rem;
  font: inherit;
  padding: 0.45rem 0.75rem;
  border: 1px solid var(--borde);
  border-radius: 6px;
  background: var(--fondo);
  color: var(--texto);
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

li {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0;
  border-top: 1px solid var(--borde);
  content-visibility: auto;
  contain-intrinsic-size: auto 74px;
}

.miniatura {
  flex: none;
  width: 96px;
  height: 54px;
  border-radius: 6px;
  overflow: hidden;
  background: var(--borde);
}

.miniatura img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.datos {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.titulo {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fecha {
  font-size: 0.85rem;
  color: var(--texto-suave);
}

.botones {
  display: flex;
  gap: 0.5rem;
  flex: none;
}

.botones button {
  padding: 0.3rem 0.75rem;
  font-size: 0.9rem;
}
</style>
