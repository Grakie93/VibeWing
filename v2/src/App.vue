<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import logoLight from './assets/vibewing-logo.png'
import logoDark from './assets/vibewing-logo-dark.png'
import ProjectCard from './components/ProjectCard.vue'
import ProjectEditor from './components/ProjectEditor.vue'
import ChatPanel from './components/ChatPanel.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import GitPanel from './components/GitPanel.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import { language, t } from './i18n'
import { desktop } from './services/desktop'
import { confirmDialog, confirmState, resolveConfirm } from './services/confirm'
import type { Project, ProjectView, UpdateInfo } from './types'

const projects = ref<ProjectView[]>([])
const dataDir = ref('')
const editorOpen = ref(false)
const editing = ref<ProjectView | null>(null)
const settings = ref<any>({
  language: 'zh-CN',
  theme: { accent: '#20bdb7', bg: '#f2fbfb', card: '#fff', preset: 'winglight' },
  check_updates: true,
  last_update_check: 0,
  default_chat_model: '',
  providers: [],
  onboarding_complete: false,
})
const chatOpen = ref(false)
const settingsOpen = ref(false)
const gitOpen = ref(false)
const gitProject = ref<ProjectView | null>(null)
const pendingLog = ref('')
const updateInfo = ref<UpdateInfo | null>(null)
const checkingUpdate = ref(false)
const logo = computed(() => (settings.value.theme?.preset === 'wingdark' ? logoDark : logoLight))
let refreshTimer: number | undefined

function applyTheme(theme: any) {
  const presets: Record<string, { accent: string; bg: string; card: string; text: string; muted: string }> = {
    winglight: { accent: '#20bdb7', bg: '#f2fbfb', card: '#ffffff', text: '#173044', muted: '#65808d' },
    wingdark: { accent: '#77d9d2', bg: '#10232e', card: '#183743', text: '#e7fbfa', muted: '#a8c7cb' },
  }
  const presetName = theme?.preset || 'winglight'
  const selected = presets[presetName] || presets.winglight
  const root = document.documentElement

  // The settings UI only edits `preset`. `accent`/`bg`/`card` come from
  // the default Theme and would otherwise override the dark preset back
  // to the light colors. Always drive background from the preset so that
  // the entire surface changes; custom palettes are reserved for future use.
  root.style.setProperty('--accent', selected.accent)
  root.style.setProperty('--app-bg', selected.bg)
  root.style.setProperty('--card-bg', selected.card)
  root.style.setProperty('--app-text', selected.text)
  root.style.setProperty('--muted', selected.muted)
  root.dataset.theme = presetName
  root.classList.toggle(
    'theme-dark',
    presetName === 'wingdark' || /^#(?:0{2}|1[0-9]|2[0-9])/i.test(theme?.bg || ''),
  )
}

async function refresh() {
  if (!document.hidden) projects.value = await desktop.listProjects()
}

async function rescan() {
  try {
    projects.value = await desktop.rescanProjects()
  } catch (error) {
    alert(`${t('app.rescanFailed')}：${String(error)}`)
  }
}

async function openDir() {
  if (!dataDir.value) return
  try {
    await desktop.openPath(dataDir.value)
  } catch (error) {
    alert(`${t('app.openDataDirFailed')}：${String(error)}`)
  }
}

function openEditor(project: ProjectView | null = null) {
  editing.value = project
  editorOpen.value = true
}

async function save(project: Project) {
  try {
    if (!project.name.trim()) throw new Error(t('editor.error.requiredName'))
    if (!project.path.trim()) throw new Error(t('editor.error.requiredPath'))
    await desktop.saveProject(project)
    editorOpen.value = false
    editing.value = null
    await refresh()
  } catch (error) {
    alert(`${t('editor.action.save')}${t('chat.requestFailed')}${String(error)}`)
  }
}

async function remove(id: string) {
  const ok = await confirmDialog({ message: t('editor.confirm.remove'), danger: true })
  if (!ok) return
  try {
    await desktop.deleteProject(id)
    editorOpen.value = false
    editing.value = null
    await refresh()
  } catch (error) {
    alert(`${t('editor.error.removeFailed')}：${String(error)}`)
  }
}

function askSelectedLog(log: string) {
  pendingLog.value = log
  chatOpen.value = true
}

async function toggleTheme() {
  const current = settings.value?.theme?.preset || 'winglight'
  const next = current === 'wingdark' ? 'winglight' : 'wingdark'
  const updated = { ...settings.value, theme: { ...settings.value.theme, preset: next } }
  try {
    await desktop.saveSettings(updated)
    settings.value = updated
    applyTheme(updated.theme)
  } catch (error) {
    alert(String(error))
  }
}

async function onMemoryUpdated(memory: string) {
  const updated = { ...settings.value, memory }
  try {
    await desktop.saveSettings(updated)
    settings.value = updated
  } catch (error) {
    alert(String(error))
  }
}

async function checkUpdate() {
  if (checkingUpdate.value) return
  checkingUpdate.value = true
  try {
    const result = await desktop.checkUpdate()
    updateInfo.value = result.has_update ? result : null
    const updated = { ...settings.value, last_update_check: Math.floor(Date.now() / 1000) }
    await desktop.saveSettings(updated)
    settings.value = updated
  } catch (error) {
    console.error('Update check failed:', error)
    updateInfo.value = null
  } finally {
    checkingUpdate.value = false
  }
}

onMounted(async () => {
  settings.value = await desktop.getSettings()
  language.value = settings.value.language
  applyTheme(settings.value.theme)
  dataDir.value = await desktop.getProjectsDir().catch(() => '')
  await refresh()
  refreshTimer = window.setInterval(refresh, 10_000)

  const last = settings.value.last_update_check || 0
  if (settings.value.check_updates && Date.now() / 1000 - last > 86_400) {
    checkUpdate()
  }
})

onBeforeUnmount(() => clearInterval(refreshTimer))
</script>

<template>
  <main>
    <header class="topbar">
      <div class="brand">
        <img :src="logo" alt="VibeWing" />
        <div>
          <div class="brand-title-row">
            <h1>VibeWing</h1>
            <small class="dev-badge">{{ t('app.brand.tagline.dev') }}</small>
            <button
              class="theme-toggle"
              type="button"
              :title="settings.theme?.preset === 'wingdark' ? t('app.theme.toLight') : t('app.theme.toDark')"
              :aria-label="t('app.theme.toggle')"
              @click="toggleTheme"
            >
              <svg
                v-if="settings.theme?.preset !== 'wingdark'"
                class="theme-icon"
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
              </svg>
              <svg
                v-else
                class="theme-icon"
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v2" />
                <path d="M12 20v2" />
                <path d="m4.93 4.93 1.41 1.41" />
                <path d="m17.66 17.66 1.41 1.41" />
                <path d="M2 12h2" />
                <path d="M20 12h2" />
                <path d="m4.93 19.07 1.41-1.41" />
                <path d="m17.66 6.34 1.41-1.41" />
              </svg>
            </button>
          </div>
          <p>{{ t('app.subtitle') }}</p>
        </div>
      </div>
      <nav>
        <button @click="chatOpen = true">💬 {{ t('app.ai') }}</button>
        <button @click="settingsOpen = true">⚙ {{ t('app.settings') }}</button>
        <button class="primary" @click="openEditor()">＋ {{ t('app.import') }}</button>
      </nav>
    </header>

    <div class="data-bar" v-if="dataDir">
      <span class="data-bar-label">{{ t('app.projectsDir') }}</span>
      <code class="data-bar-path" :title="dataDir">{{ dataDir }}</code>
      <button type="button" @click="rescan" :title="t('app.rescan')">{{ t('app.rescan') }}</button>
      <button type="button" @click="openDir" :title="t('app.openDataDir')">{{ t('app.openDataDir') }}</button>
    </div>

    <div v-if="updateInfo" class="update-banner">
      <span>{{ t('app.updateAvailable', { current: updateInfo.current_version, latest: updateInfo.latest_version }) }}</span>
      <button type="button" @click="desktop.openUrl(updateInfo.html_url)">{{ t('app.updateButton') }}</button>
    </div>

    <section v-if="projects.length" class="project-grid">
      <ProjectCard
        v-for="project in projects"
        :key="project.id"
        :project="project"
        @edit="openEditor"
        @changed="refresh"
        @git="(p: ProjectView) => { gitProject = p; gitOpen = true }"
        @ask-ai="askSelectedLog"
      />
    </section>
    <div v-else class="empty">{{ t('app.empty') }}</div>

    <ProjectEditor
      :open="editorOpen"
      :project="editing"
      @close="editorOpen = false"
      @save="save"
      @remove="remove"
    />

    <div v-if="chatOpen" class="modal chat-modal" @mousedown.self="chatOpen = false">
      <section class="dialog chat-dialog">
        <header>
          <h2>{{ t('chat.title') }}</h2>
          <button type="button" @click="chatOpen = false">×</button>
        </header>
        <ChatPanel :settings="settings" :projects="projects" :pending-log="pendingLog" @memory-updated="onMemoryUpdated" />
      </section>
    </div>

    <SettingsPanel
      :open="settingsOpen"
      :settings="settings"
      @close="settingsOpen = false"
      @saved="(s) => { settings = s; language = s.language; applyTheme(s.theme) }"
    />

    <div v-if="gitOpen && gitProject" class="modal git-modal" @mousedown.self="gitOpen = false">
      <section class="dialog git-dialog">
        <header>
          <h2>{{ t('git.title', { name: gitProject.name }) }}</h2>
          <button type="button" @click="gitOpen = false">×</button>
        </header>
        <GitPanel :project="gitProject" :settings="settings" />
      </section>
    </div>

    <ConfirmDialog
      :open="confirmState.open"
      :title="confirmState.title"
      :message="confirmState.message"
      :confirm-text="confirmState.confirmText"
      :cancel-text="confirmState.cancelText"
      :danger="confirmState.danger"
      @confirm="resolveConfirm(true)"
      @cancel="resolveConfirm(false)"
    />
  </main>
</template>
