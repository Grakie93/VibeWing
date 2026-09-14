<script setup lang="ts">
import { computed } from 'vue'
import { t } from '../i18n'
import type { ProjectView } from '../types'
import { Pin } from '@lucide/vue'
import ServicePanel from './ServicePanel.vue'

const props = defineProps<{ project: ProjectView; pinned?: boolean }>()
const emit = defineEmits<{
  edit: [project: ProjectView]
  changed: []
  git: [project: ProjectView]
  askAi: [log: string]
  togglePin: [id: string]
}>()
const isPinned = computed(() => Boolean(props.pinned))
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
        <button
          type="button"
          class="pin-btn"
          :class="{ pinned: isPinned }"
          :data-tip="isPinned ? t('project.unpin') : t('project.pin')"
          :aria-label="isPinned ? t('project.unpin') : t('project.pin')"
          @click="emit('togglePin', project.id)"
        ><Pin /></button>
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
