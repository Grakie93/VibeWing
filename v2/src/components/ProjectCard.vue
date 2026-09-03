<script setup lang="ts">
import { t } from '../i18n'
import type { ProjectView } from '../types'
import ServicePanel from './ServicePanel.vue'

defineProps<{ project: ProjectView }>()
const emit = defineEmits<{
  edit: [project: ProjectView]
  changed: []
  git: [project: ProjectView]
  askAi: [log: string]
}>()
</script>

<template>
  <article class="project-card">
    <header class="project-header">
      <div>
        <h2>
          {{ project.name }}
          <span v-if="project.source === 'file'" class="source-badge" :title="t('project.importedHint')">{{ t('project.imported') }}</span>
        </h2>
        <p>{{ project.path }}</p>
      </div>
      <div class="card-actions">
        <button @click="emit('edit', project)">{{ t('editor.title.edit') }}</button>
        <button @click="emit('git', project)">Git</button>
      </div>
    </header>
    <ServicePanel
      :project="project"
      service="frontend"
      @changed="emit('changed')"
      @ask-ai="emit('askAi', $event)"
    />
    <ServicePanel
      :project="project"
      service="backend"
      @changed="emit('changed')"
      @ask-ai="emit('askAi', $event)"
    />
  </article>
</template>
