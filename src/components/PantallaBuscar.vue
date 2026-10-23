<script setup lang="ts">
import { ref } from "vue";
import ListaVideos from "./ListaVideos.vue";
import { ajustes } from "../ajustes";
import { buscarVideos, cancelarConsulta, type Elemento } from "../analisis";
import { agregarACola } from "../descargas";

const POR_PAGINA = 20;

const emit = defineEmits<{ agregados: [cantidad: number] }>();

const texto = ref("");
/** Lo que se buscó, para "Cargar más" aunque se edite el campo. */
const buscado = ref("");
const cantidad = ref(POR_PAGINA);
const buscando = ref(false);
const agregando = ref(false);
const error = ref<string | null>(null);
const resultados = ref<Elemento[] | null>(null);
const seleccion = ref(new Set<string>());

async function ejecutar(consulta: string, total: number) {
  buscando.value = true;
  error.value = null;
  try {
    // yt-dlp no pagina: pide todos los resultados de nuevo, hasta `total`.
    resultados.value = await buscarVideos(consulta, total, ajustes.carpetaDestino);
    buscado.value = consulta;
    cantidad.value = total;
    if (resultados.value.length === 0) error.value = "No se encontraron videos.";
  } catch (e) {
    error.value = String(e);
  } finally {
    buscando.value = false;
  }
}

function buscar() {
  if (!texto.value.trim() || buscando.value) return;
  resultados.value = null;
  seleccion.value = new Set();
  ejecutar(texto.value.trim(), POR_PAGINA);
}

function cargarMas() {
  ejecutar(buscado.value, cantidad.value + POR_PAGINA);
}

async function agregar() {
  if (!resultados.value) return;
  agregando.value = true;
  error.value = null;
  try {
    const elegidos = resultados.value.filter((e) => seleccion.value.has(e.id));
    const agregados = await agregarACola(elegidos, ajustes);
    seleccion.value = new Set();
    emit("agregados", agregados);
  } catch (e) {
    error.value = String(e);
  } finally {
    agregando.value = false;
  }
}
</script>

<template>
  <!-- Una sola raíz: así v-show puede ocultar la pantalla. -->
  <div>
    <section class="tarjeta">
      <form class="buscador" @submit.prevent="buscar">
        <input
          v-model="texto"
          type="search"
          placeholder="Busca videos en YouTube"
          :disabled="buscando"
          spellcheck="false"
        />
        <button v-if="!buscando" type="submit" :disabled="!texto.trim()">Buscar</button>
        <button v-else type="button" class="secundario" @click="cancelarConsulta('busqueda')">
          Cancelar
        </button>
      </form>
      <p class="suave nota">
        Busca con yt-dlp directamente en YouTube, sin cuentas ni claves de API.
      </p>
      <p v-if="buscando && !resultados" class="suave">Buscando…</p>
      <p v-if="error" class="error">{{ error }}</p>
    </section>

    <section v-if="resultados && resultados.length" class="tarjeta">
      <h2>Resultados para “{{ buscado }}”</h2>

      <ListaVideos v-model="seleccion" :elementos="resultados" mostrar-canal />

      <div v-if="resultados.length >= cantidad" class="mas">
        <button class="secundario" :disabled="buscando" @click="cargarMas">
          {{ buscando ? "Cargando…" : "Cargar más resultados" }}
        </button>
      </div>

      <footer class="pie">
        <span>{{ seleccion.size }} seleccionados</span>
        <button :disabled="seleccion.size === 0 || agregando" @click="agregar">
          Agregar a la cola
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.nota {
  margin: 0.75rem 0 0;
  font-size: 0.9rem;
}

.suave {
  margin: 1rem 0 0;
}

.mas {
  display: flex;
  justify-content: center;
  padding: 1rem 0;
  border-top: 1px solid var(--borde);
}
</style>
