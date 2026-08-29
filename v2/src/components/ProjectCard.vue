<script setup lang="ts">
import { text } from '../i18n'
import type { ProjectView } from '../types'
import ServicePanel from './ServicePanel.vue'
import GitPanel from './GitPanel.vue'

defineProps<{ project: ProjectView }>()
const emit = defineEmits<{ edit: [project: ProjectView]; changed: [] }>()
</script>

<template>
  <article class="project-card">
    <header><div><h2>{{ project.name }}</h2><p>{{ project.path }}</p></div><button @click="emit('edit', project)">{{ text.edit }}</button></header>
    <ServicePanel :project="project" service="frontend" @changed="emit('changed')" />
    <ServicePanel :project="project" service="backend" @changed="emit('changed')" />
    <GitPanel :project="project" />
  </article>
</template>
