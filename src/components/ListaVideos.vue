<script setup lang="ts">
import { computed, ref } from "vue";
import { formatoDuracion, sePuedeDescargar, type Elemento } from "../analisis";

const props = defineProps<{ elementos: Elemento[] }>();
/** IDs seleccionados. */
const seleccion = defineModel<Set<string>>({ required: true });

const ETIQUETAS: Partial<Record<Elemento["estado"], string>> = {
  respaldado: "Ya respaldado",
  en_vivo: "En vivo",
  programado: "Programado",
};

const seccion = ref<string | null>(null);

const secciones = computed(() => {
  const cuentas = new Map<string, number>();
  for (const e of props.elementos) {
    if (e.seccion) cuentas.set(e.seccion, (cuentas.get(e.seccion) ?? 0) + 1);
  }
  return [...cuentas];
});

const visibles = computed(() =>
  seccion.value === null
    ? props.elementos
    : props.elementos.filter((e) => e.seccion === seccion.value),
);

function cambiar(e: Elemento, marcado: boolean) {
  if (marcado) seleccion.value.add(e.id);
  else seleccion.value.delete(e.id);
}

function seleccionarVisibles(filtro: (e: Elemento) => boolean) {
  for (const e of visibles.value) {
    if (sePuedeDescargar(e) && filtro(e)) seleccion.value.add(e.id);
    else seleccion.value.delete(e.id);
  }
}
</script>

<template>
  <div class="lista-videos">
    <div v-if="secciones.length > 1" class="secciones">
      <button class="chip" :class="{ activa: seccion === null }" @click="seccion = null">
        Todo ({{ elementos.length }})
      </button>
      <button
        v-for="[nombre, cuenta] in secciones"
        :key="nombre"
        class="chip"
        :class="{ activa: seccion === nombre }"
        @click="seccion = nombre"
      >
        {{ nombre }} ({{ cuenta }})
      </button>
    </div>

    <div v-if="elementos.length > 1" class="acciones">
      <button class="enlace" @click="seleccionarVisibles((e) => e.estado === 'disponible')">
        Solo los nuevos
      </button>
      <button class="enlace" @click="seleccionarVisibles(() => true)">Todos</button>
      <button class="enlace" @click="seleccionarVisibles(() => false)">Ninguno</button>
    </div>

    <ul>
      <li v-for="e in visibles" :key="e.id" :class="{ inactivo: !sePuedeDescargar(e) }">
        <label>
          <input
            type="checkbox"
            :checked="seleccion.has(e.id)"
            :disabled="!sePuedeDescargar(e)"
            @change="cambiar(e, ($event.target as HTMLInputElement).checked)"
          />
          <span class="miniatura">
            <img v-if="e.miniatura" :src="e.miniatura" alt="" loading="lazy" />
            <span v-if="e.duracion" class="duracion">{{ formatoDuracion(e.duracion) }}</span>
          </span>
          <span class="datos">
            <span class="titulo">{{ e.titulo }}</span>
            <span class="detalle">
              <span v-if="e.seccion && seccion === null">{{ e.seccion }}</span>
              <span v-if="ETIQUETAS[e.estado]" class="etiqueta" :class="e.estado">
                {{ ETIQUETAS[e.estado] }}
              </span>
            </span>
          </span>
        </label>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.secciones {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.chip {
  padding: 0.25rem 0.75rem;
  border-radius: 999px;
  border: 1px solid var(--borde);
  background: transparent;
  color: var(--texto);
  font-size: 0.9rem;
}

.chip:hover:not(:disabled) {
  background: var(--fondo);
}

.chip.activa {
  background: var(--acento);
  border-color: var(--acento);
  color: #fff;
}

.acciones {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  font-size: 0.9rem;
}

ul {
  list-style: none;
  margin: 0;
  padding: 0;
}

li {
  border-top: 1px solid var(--borde);
  /* Con canales de miles de videos, el navegador no dibuja las filas fuera de vista. */
  content-visibility: auto;
  contain-intrinsic-size: auto 82px;
}

li.inactivo {
  opacity: 0.55;
}

label {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.5rem 0.25rem;
  cursor: pointer;
}

li.inactivo label {
  cursor: default;
}

.miniatura {
  position: relative;
  flex: none;
  width: 116px;
  height: 65px;
  border-radius: 6px;
  overflow: hidden;
  background: var(--borde);
}

.miniatura img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.duracion {
  position: absolute;
  right: 4px;
  bottom: 4px;
  padding: 0 4px;
  border-radius: 3px;
  background: rgba(0, 0, 0, 0.8);
  color: #fff;
  font-size: 0.75rem;
}

.datos {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.titulo {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.detalle {
  display: flex;
  gap: 0.5rem;
  color: var(--texto-suave);
  font-size: 0.85rem;
}

.etiqueta {
  padding: 0 0.4rem;
  border-radius: 4px;
  background: var(--fondo);
}

.etiqueta.respaldado {
  color: var(--exito);
}
</style>
