<script setup lang="ts">
import { ref } from 'vue'
import { text } from '../i18n'
import { desktop } from '../services/desktop'
import type { ProjectView, ServiceKind } from '../types'

const props = defineProps<{ project: ProjectView; service: ServiceKind }>()
const emit = defineEmits<{ changed: []; askAi: [log: string] }>()
const busy = ref('')
const log = ref('')
const logOpen = ref(false)
const selectedLog = ref('')

const value = (suffix: string) => props.project[`${props.service}_${suffix}` as keyof ProjectView]

async function action(name: 'start' | 'stop' | 'restart') {
  busy.value = name
  try { await desktop.serviceAction(props.project.id, props.service, name); emit('changed') }
  catch (error) { alert(String(error)) }
  finally { busy.value = '' }
}

async function toggleLog() {
  logOpen.value = !logOpen.value
  if (logOpen.value) log.value = await desktop.readLog(props.project.id, props.service)
}
function onLogSelection() {
  const selection = window.getSelection()?.toString().trim() || ''
  selectedLog.value = selection.slice(0, 5000)
}
function askSelected() { if (selectedLog.value) emit('askAi', selectedLog.value); selectedLog.value = '' }
</script>

<template>
  <section class="service-panel">
    <div class="service-title">
      <strong><i :class="['status-dot', { running: value('running') }]" />{{ text[service] }}</strong>
      <a v-if="value('port')" class="port-link" :href="`http://localhost:${value('port')}`" target="_blank" rel="noreferrer">Port {{ value('port') }} ↗</a><span v-else>—</span>
    </div>
    <p>{{ value('path') || project.path }}</p>
    <code>{{ value('cmd') || '—' }}</code>
    <div class="actions">
      <button :disabled="!!busy" @click="action('start')">{{ text.start }}</button>
      <button :disabled="!!busy" @click="action('restart')">{{ text.restart }}</button>
      <button :disabled="!!busy" @click="action('stop')">{{ text.stop }}</button>
      <button @click="toggleLog">{{ text.logs }}</button>
    </div>
    <div v-if="logOpen" class="log-view"><pre @mouseup="onLogSelection">{{ log || '暂无日志' }}</pre><button v-if="selectedLog" type="button" class="ask-log-button" @click="askSelected">问问 AI</button></div>
  </section>
</template>
