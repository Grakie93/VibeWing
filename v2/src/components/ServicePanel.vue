<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { t } from '../i18n'
import { desktop } from '../services/desktop'
import type { ProjectView, ServiceKind } from '../types'

const props = defineProps<{ project: ProjectView; service: ServiceKind }>()
const emit = defineEmits<{ changed: []; askAi: [log: string] }>()
const busy = ref('')
const log = ref('')
const logOpen = ref(false)
const selectedLog = ref('')
const toolbarVisible = ref(false)
const toolbarPos = ref({ x: 0, y: 0 })
const contextMenuVisible = ref(false)
const contextMenuPos = ref({ x: 0, y: 0 })
const logRef = ref<HTMLElement | null>(null)
const buildMenuOpen = ref(false)
let logInterval: number | null = null
let startInterval: number | null = null

const value = (suffix: string) => props.project[`${props.service}_${suffix}` as keyof ProjectView]
const isStarting = computed(() => Boolean(value('starting')))

// While a service is "starting" (pid alive, port not yet open) we can't rely on
// the 10s global refresh to flip the light to green promptly, so poll the
// project list locally until the service is ready or stopped.
watch(
  isStarting,
  starting => {
    if (starting && startInterval === null) {
      startInterval = window.setInterval(() => emit('changed'), 1500)
    } else if (!starting && startInterval !== null) {
      clearInterval(startInterval)
      startInterval = null
    }
  },
  { immediate: true },
)

function inferBuildCommands(cmd: string): { build?: string; testBuild?: string } {
  const c = cmd.trim().toLowerCase()
  if (!c || c.includes('python') || c.includes('.venv')) return {}
  if (c.startsWith('npm ')) return { build: 'npm run build', testBuild: 'npm run build:test' }
  if (c.startsWith('pnpm ')) return { build: 'pnpm build', testBuild: 'pnpm build:test' }
  if (c.startsWith('yarn ')) return { build: 'yarn build', testBuild: 'yarn build:test' }
  if (c.startsWith('cargo ')) return { build: 'cargo build', testBuild: 'cargo test' }
  if (c.startsWith('go ')) return { build: 'go build .', testBuild: 'go test ./...' }
  if (c.startsWith('mvn ')) return { build: 'mvn package', testBuild: 'mvn test' }
  return {}
}

const runCmd = computed(() => String(value('cmd') || ''))
const inferred = computed(() => inferBuildCommands(runCmd.value))
const storedBuild = computed(() => String(value('build') || '').trim())
const storedTestBuild = computed(() => String(value('test_build') || '').trim())
const effectiveBuild = computed(() => storedBuild.value || inferred.value.build || '')
const effectiveTestBuild = computed(() => storedTestBuild.value || inferred.value.testBuild || '')
const canBuild = computed(() => Boolean(effectiveBuild.value || effectiveTestBuild.value))

function buildTitle(test: boolean): string {
  const stored = test ? storedTestBuild.value : storedBuild.value
  const inferredCmd = test ? inferred.value.testBuild : inferred.value.build
  if (stored) return ''
  if (inferredCmd) return `${t('service.buildWillUse')}${inferredCmd}`
  return t(test ? 'service.buildTestEmpty' : 'service.buildEmpty')
}

async function action(name: 'start' | 'stop' | 'restart') {
  busy.value = name
  try {
    await desktop.serviceAction(props.project.id, props.service, name)
    emit('changed')
  } catch (error) {
    alert(String(error))
  } finally {
    busy.value = ''
  }
}

async function build(test: boolean) {
  busy.value = test ? 'build-test' : 'build'
  try {
    await desktop.buildProject(props.project.id, props.service, test)
    emit('changed')
  } catch (error) {
    alert(String(error))
  } finally {
    busy.value = ''
  }
}

function toggleBuildMenu() {
  buildMenuOpen.value = !buildMenuOpen.value
}

function closeBuildMenu() {
  buildMenuOpen.value = false
}

async function runBuild(test: boolean) {
  buildMenuOpen.value = false
  await build(test)
}

async function toggleLog() {
  logOpen.value = !logOpen.value
  if (logOpen.value) {
    await refreshLog()
    startLogPolling()
  } else {
    stopLogPolling()
  }
}

async function refreshLog() {
  if (!logOpen.value) return
  log.value = await desktop.readLog(props.project.id, props.service)
}

function startLogPolling() {
  stopLogPolling()
  logInterval = window.setInterval(() => refreshLog().catch(() => undefined), 800)
}

function stopLogPolling() {
  if (logInterval !== null) {
    clearInterval(logInterval)
    logInterval = null
  }
}

async function clearLog() {
  await desktop.clearLog(props.project.id, props.service)
  log.value = ''
}

function readSelection() {
  const selection = window.getSelection()?.toString().trim() || ''
  return selection.slice(0, 5000)
}

function selectionRect() {
  const selection = window.getSelection()
  if (!selection || selection.rangeCount === 0) return null
  return selection.getRangeAt(0).getBoundingClientRect()
}

function updateToolbar() {
  window.setTimeout(() => {
    const text = readSelection()
    if (!text) {
      toolbarVisible.value = false
      return
    }
    selectedLog.value = text
    const rect = selectionRect()
    const container = logRef.value?.getBoundingClientRect()
    if (rect && container) {
      toolbarPos.value = {
        x: rect.left - container.left + rect.width / 2,
        y: rect.top - container.top - 36,
      }
    }
    toolbarVisible.value = true
  }, 0)
}

function hideToolbarIfOutside(event: MouseEvent) {
  const target = event.target as Node
  if (!(target instanceof Element) || !target.closest('.log-view')) {
    toolbarVisible.value = false
    contextMenuVisible.value = false
  }
}

function hideContextMenu(event: MouseEvent) {
  const target = event.target as Node
  if (target instanceof Element && target.closest('.selection-context-menu')) return
  contextMenuVisible.value = false
}

function askSelected() {
  if (selectedLog.value) emit('askAi', selectedLog.value)
  toolbarVisible.value = false
  contextMenuVisible.value = false
}

function copySelected() {
  if (selectedLog.value) navigator.clipboard.writeText(selectedLog.value).catch(() => undefined)
  contextMenuVisible.value = false
}

function onContextMenu(event: MouseEvent) {
  const text = readSelection()
  if (!text) return
  event.preventDefault()
  selectedLog.value = text
  const container = logRef.value?.getBoundingClientRect()
  if (container) {
    contextMenuPos.value = {
      x: event.clientX - container.left,
      y: event.clientY - container.top,
    }
  }
  contextMenuVisible.value = true
}

function openPort() {
  const port = value('port')
  if (port) desktop.openUrl(`http://localhost:${port}`).catch(error => alert(String(error)))
}

window.addEventListener('mousedown', hideToolbarIfOutside)
window.addEventListener('click', hideContextMenu)
window.addEventListener('click', closeBuildMenu)
onBeforeUnmount(() => {
  window.removeEventListener('mousedown', hideToolbarIfOutside)
  window.removeEventListener('click', hideContextMenu)
  window.removeEventListener('click', closeBuildMenu)
  stopLogPolling()
  if (startInterval !== null) {
    clearInterval(startInterval)
    startInterval = null
  }
})
</script>

<template>
  <section class="service-panel">
    <div class="service-title">
      <strong>
        <i :class="['status-dot', { running: value('running'), starting: value('starting') }]" />
        {{ t(`service.${service}`) }}
        <span v-if="value('starting')" class="status-text starting">{{ t('service.starting') }}</span>
      </strong>
      <button
        v-if="value('port')"
        type="button"
        class="port-link"
        :title="t('service.logs')"
        @click="openPort"
      >
        Port {{ value('port') }} ↗
      </button>
      <span v-else>—</span>
    </div>
    <p>{{ value('path') || project.path }}</p>
    <code>{{ value('cmd') || '—' }}</code>
    <div class="actions">
      <button :disabled="!!busy" @click="action('start')">{{ t('service.start') }}</button>
      <button :disabled="!!busy" @click="action('restart')">{{ t('service.restart') }}</button>
      <button :disabled="!!busy" @click="action('stop')">{{ t('service.stop') }}</button>
      <div class="build-menu-wrap">
        <button :disabled="!canBuild || !!busy" @click.stop="toggleBuildMenu">
          <span class="build-caret">▾</span> {{ t('service.build') }}
        </button>
        <div v-if="buildMenuOpen" class="build-menu" @click.stop>
          <button
            :disabled="!effectiveBuild || !!busy"
            :title="buildTitle(false)"
            @click="runBuild(false)"
          >{{ t('service.buildProd') }}</button>
          <button
            :disabled="!effectiveTestBuild || !!busy"
            :title="buildTitle(true)"
            @click="runBuild(true)"
          >{{ t('service.buildTest') }}</button>
        </div>
      </div>
      <button @click="toggleLog">{{ t('service.logs') }}</button>
    </div>
    <div v-if="logOpen" ref="logRef" class="log-view">
      <div class="log-header">
        <span>{{ t('service.logs') }}</span>
        <button type="button" class="log-clear" @click="clearLog">{{ t('service.clearLogs') }}</button>
      </div>
      <pre @mouseup="updateToolbar" @contextmenu="onContextMenu">{{ log || t('service.emptyLogs') }}</pre>
      <div
        v-if="toolbarVisible"
        class="selection-toolbar"
        :style="{ left: `${toolbarPos.x}px`, top: `${toolbarPos.y}px` }"
      >
        <button type="button" @mousedown.prevent="askSelected">{{ t('service.askAi') }}</button>
      </div>
      <div
        v-if="contextMenuVisible"
        class="selection-context-menu"
        :style="{ left: `${contextMenuPos.x}px`, top: `${contextMenuPos.y}px` }"
      >
        <button type="button" @mousedown.prevent="copySelected">{{ t('chat.context.copy') }}</button>
        <button type="button" @mousedown.prevent="askSelected">{{ t('chat.context.askAi') }}</button>
      </div>
    </div>
  </section>
</template>
