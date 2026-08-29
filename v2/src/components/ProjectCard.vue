<script setup lang="ts">
import { text } from '../i18n'
import type { ProjectView } from '../types'
import ServicePanel from './ServicePanel.vue'

defineProps<{ project: ProjectView }>()
const emit = defineEmits<{ edit: [project: ProjectView]; changed: []; git: [project: ProjectView] }>()
</script>

<template>
  <article class="project-card">
    <header><div><h2>{{ project.name }}</h2><p>{{ project.path }}</p></div><button @click="emit('edit', project)">{{ text.edit }}</button></header>
    <ServicePanel :project="project" service="frontend" @changed="emit('changed')" />
    <ServicePanel :project="project" service="backend" @changed="emit('changed')" />
    <button class="git-open" @click="emit('git', project)">Git</button>
  </article>
</template>
