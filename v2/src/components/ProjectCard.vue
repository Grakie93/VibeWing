<script setup lang="ts">
import { text } from '../i18n'
import type { ProjectView } from '../types'
import ServicePanel from './ServicePanel.vue'

defineProps<{ project: ProjectView }>()
const emit = defineEmits<{ edit: [project: ProjectView]; changed: []; git: [project: ProjectView]; remove: [id: string] }>()
</script>

<template>
  <article class="project-card">
    <header class="project-header">
      <div><h2>{{ project.name }}</h2><p>{{ project.path }}</p></div>
      <div class="card-actions"><button @click="emit('edit', project)">{{ text.edit }}</button><button @click="emit('git', project)">Git</button><button class="danger-icon" title="移除项目" @click="emit('remove', project.id)">×</button></div>
    </header>
    <ServicePanel :project="project" service="frontend" @changed="emit('changed')" />
    <ServicePanel :project="project" service="backend" @changed="emit('changed')" />
  </article>
</template>
