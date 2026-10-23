<script setup lang="ts">
import { computed } from "vue";
import { porcentaje, textoProgreso, type Progreso } from "../herramientas";

const props = defineProps<{ nombre: string; progreso: Progreso | null; version?: string | null }>();

const valor = computed(() => (props.progreso ? porcentaje(props.progreso) : null));
const texto = computed(() => {
  if (props.progreso) return textoProgreso(props.progreso);
  return props.version ? `Instalado (${props.version})` : "En espera";
});
</script>

<template>
  <div class="herramienta">
    <div class="encabezado">
      <strong>{{ nombre }}</strong>
      <span class="texto">{{ texto }}</span>
    </div>
    <div class="barra" :class="{ indeterminada: progreso && valor === null }">
      <div class="relleno" :style="{ width: `${valor ?? (version ? 100 : 0)}%` }"></div>
    </div>
  </div>
</template>

<style scoped>
.herramienta {
  margin-bottom: 1.25rem;
}

.encabezado {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 0.35rem;
}

.texto {
  color: var(--texto-suave);
  font-size: 0.9rem;
}

.barra {
  height: 8px;
  border-radius: 4px;
  background: var(--borde);
  overflow: hidden;
}

.relleno {
  height: 100%;
  background: var(--acento);
  transition: width 0.2s;
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
