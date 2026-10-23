<script setup lang="ts">
import { computed } from "vue";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  cancelarDescarga,
  cola,
  limpiarTerminados,
  pendientes,
  quitarDeCola,
  reintentarDescarga,
  textoEstado,
} from "../descargas";

const terminados = computed(() => cola.filter((d) => d.estado === "terminado").length);
const conError = computed(() => cola.filter((d) => d.estado === "error").length);
</script>

<template>
  <section class="tarjeta">
    <header class="encabezado">
      <div>
        <h2>Cola de descargas</h2>
        <p v-if="cola.length" class="suave">
          {{ pendientes }} pendientes · {{ terminados }} terminados
          <template v-if="conError"> · {{ conError }} con error</template>
        </p>
      </div>
      <button v-if="terminados" class="secundario" @click="limpiarTerminados">
        Quitar terminados
      </button>
    </header>

    <p v-if="!cola.length" class="suave vacio">
      La cola está vacía. Analiza un enlace en la pestaña Descargar y agrega videos.
    </p>

    <ul>
      <li v-for="d in cola" :key="d.id" :class="d.estado">
        <span class="miniatura">
          <img v-if="d.miniatura" :src="d.miniatura" alt="" loading="lazy" />
        </span>
        <div class="datos">
          <span class="titulo">{{ d.titulo }}</span>
          <span class="estado">{{ textoEstado(d) }}</span>
          <div
            v-if="d.estado === 'descargando' || d.estado === 'procesando'"
            class="barra"
            :class="{ indeterminada: d.estado === 'procesando' || d.porcentaje === null }"
          >
            <div class="relleno" :style="{ width: `${d.porcentaje ?? 0}%` }"></div>
          </div>
        </div>
        <div class="botones">
          <button
            v-if="['en_espera', 'descargando', 'procesando'].includes(d.estado)"
            class="secundario"
            @click="cancelarDescarga(d.id)"
          >
            Cancelar
          </button>
          <button v-if="d.estado === 'error' || d.estado === 'cancelado'" @click="reintentarDescarga(d.id)">
            Reintentar
          </button>
          <button v-if="d.archivo" class="secundario" @click="revealItemInDir(d.archivo)">
            Mostrar
          </button>
          <button
            v-if="['terminado', 'error', 'cancelado'].includes(d.estado)"
            class="secundario quitar"
            title="Quitar de la lista"
            @click="quitarDeCola(d.id)"
          >
            ×
          </button>
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

.vacio {
  margin: 0;
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
  contain-intrinsic-size: auto 80px;
}

.miniatura {
  flex: none;
  width: 104px;
  height: 58px;
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
  gap: 0.2rem;
}

.titulo {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.estado {
  font-size: 0.85rem;
  color: var(--texto-suave);
}

.terminado .estado {
  color: var(--exito);
}

.error .estado {
  color: var(--error-texto);
}

.cancelado,
.en_espera {
  opacity: 0.75;
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

.quitar {
  font-size: 1.1rem;
  line-height: 1;
}

.barra {
  height: 6px;
  border-radius: 3px;
  background: var(--borde);
  overflow: hidden;
}

.relleno {
  height: 100%;
  background: var(--acento);
  transition: width 0.25s;
}

.indeterminada .relleno {
  width: 35% !important;
  animation: vaiven 1.2s ease-in-out infinite;
}

@keyframes vaiven {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(300%);
  }
}
</style>
