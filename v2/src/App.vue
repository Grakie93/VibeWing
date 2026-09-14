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
import NotifyContainer from './components/NotifyContainer.vue'
import { language, t } from './i18n'
import { Search, MessageSquare, Settings, Plus, X } from '@lucide/vue'
import { desktop } from './services/desktop'
import { confirmDialog, confirmState, resolveConfirm } from './services/confirm'
import { initInstantTooltips } from './services/tooltip'
import type { Project, ProjectView, UpdateInfo } from './types'

const updateModalOpen = ref(false)

const projects = ref<ProjectView[]>([])
const searchQuery = ref('')
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

function sortProjects(list: ProjectView[]): ProjectView[] {
  const pinned: string[] = settings.value.pinned_ids || []
  const pinnedSet = new Set(pinned)
  const top = pinned
    .map(id => list.find(p => p.id === id))
    .filter((p): p is ProjectView => Boolean(p))
  const rest = list.filter(p => !pinnedSet.has(p.id))
  return [...top, ...rest]
}

const visibleProjects = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return projects.value
  return projects.value.filter(
    p => p.name.toLowerCase().includes(q) || (p.path || '').toLowerCase().includes(q),
  )
})

async function refresh() {
  if (!document.hidden) projects.value = sortProjects(await desktop.listProjects())
}

async function rescan() {
  try {
    projects.value = sortProjects(await desktop.rescanProjects())
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

async function togglePin(id: string) {
  const current: string[] = settings.value.pinned_ids || []
  const next = current.includes(id)
    ? current.filter((x: string) => x !== id)
    : [...current, id]
  const updated = { ...settings.value, pinned_ids: next }
  try {
    await desktop.saveSettings(updated)
    settings.value = updated
    projects.value = sortProjects(projects.value)
  } catch (error) {
    alert(`${t('project.pin')}${t('chat.requestFailed')}${String(error)}`)
  }
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
    refreshUpdateModal()
  }
}

function refreshUpdateModal() {
  const dismissed = settings.value.update_dismissed_version
  updateModalOpen.value = !!updateInfo.value && updateInfo.value.latest_version !== dismissed
}

async function ignoreUpdate() {
  if (!updateInfo.value) return
  const updated = { ...settings.value, update_dismissed_version: updateInfo.value.latest_version }
  await desktop.saveSettings(updated)
  settings.value = updated
  updateModalOpen.value = false
}

function laterUpdate() {
  updateModalOpen.value = false
}

async function goUpdate() {
  if (!updateInfo.value) return
  const url = updateInfo.value.html_url
  const updated = { ...settings.value, update_dismissed_version: updateInfo.value.latest_version }
  await desktop.saveSettings(updated)
  settings.value = updated
  updateModalOpen.value = false
  await desktop.openUrl(url)
}

function openUpdateUrl() {
  if (updateInfo.value) desktop.openUrl(updateInfo.value.html_url)
}

onMounted(async () => {
  initInstantTooltips()
  settings.value = await desktop.getSettings()
  language.value = settings.value.language
  applyTheme(settings.value.theme)
  dataDir.value = await desktop.getProjectsDir().catch(() => '')
  await refresh()
  refreshTimer = window.setInterval(refresh, 10_000)

  if (settings.value.check_updates) {
    checkUpdate()
  }
})

onBeforeUnmount(() => clearInterval(refreshTimer))
</script>

<template>
  <main>
    <NotifyContainer />
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
        <div class="search-box">
          <span class="search-icon" aria-hidden="true"><Search /></span>
          <input
            v-model="searchQuery"
            type="search"
            class="search-input"
            :placeholder="t('app.searchPlaceholder')"
          />
          <button
            type="button"
            class="search-clear"
            :style="{ visibility: searchQuery ? 'visible' : 'hidden' }"
            :aria-label="t('app.searchClear')"
            @click="searchQuery = ''"
          ><X /></button>
        </div>
        <button @click="chatOpen = true"><MessageSquare /> {{ t('app.ai') }}</button>
        <button @click="settingsOpen = true"><Settings /> {{ t('app.settings') }}</button>
        <button class="primary" @click="openEditor()"><Plus /> {{ t('app.import') }}</button>
      </nav>
    </header>

    <div class="data-bar" v-if="dataDir">
      <span class="data-bar-label">{{ t('app.projectsDir') }}</span>
      <code class="data-bar-path">{{ dataDir }}</code>
      <button type="button" @click="rescan">{{ t('app.rescan') }}</button>
      <button type="button" @click="openDir">{{ t('app.openDataDir') }}</button>
    </div>

    <div v-if="updateModalOpen && updateInfo" class="modal update-modal" @mousedown.self="laterUpdate">
      <section class="dialog update-dialog">
        <h2>{{ t('app.updateTitle') }}</h2>
        <p>{{ t('app.updateAvailable', { current: updateInfo.current_version, latest: updateInfo.latest_version }) }}</p>
        <div class="update-actions">
          <button type="button" @click="ignoreUpdate">{{ t('app.updateIgnore') }}</button>
          <button type="button" @click="laterUpdate">{{ t('app.updateLater') }}</button>
          <button type="button" class="primary" @click="goUpdate">{{ t('app.updateButton') }}</button>
        </div>
      </section>
    </div>

    <section v-if="visibleProjects.length" class="project-grid">
      <ProjectCard
        v-for="project in visibleProjects"
        :key="project.id"
        :project="project"
        :pinned="settings.pinned_ids?.includes(project.id)"
        @edit="openEditor"
        @changed="refresh"
        @git="(p: ProjectView) => { gitProject = p; gitOpen = true }"
        @ask-ai="askSelectedLog"
        @toggle-pin="togglePin"
      />
    </section>
    <div v-else-if="searchQuery" class="empty">{{ t('app.searchEmpty') }}</div>
    <div v-else class="empty">{{ t('app.empty') }}</div>

    <ProjectEditor
      :open="editorOpen"
      :project="editing"
      @close="editorOpen = false"
      @save="save"
      @remove="remove"
    />

    <div v-show="chatOpen" class="modal chat-modal" @mousedown.self="chatOpen = false">
      <section class="dialog chat-dialog">
        <header>
          <h2>{{ t('chat.title') }}</h2>
          <button type="button" @click="chatOpen = false"><X /></button>
        </header>
        <ChatPanel :settings="settings" :projects="projects" :pending-log="pendingLog" @memory-updated="onMemoryUpdated" />
      </section>
    </div>

    <SettingsPanel
      :open="settingsOpen"
      :settings="settings"
      :update-info="updateInfo"
      @close="settingsOpen = false"
      @download-update="openUpdateUrl"
      @saved="(s) => { settings = s; language = s.language; applyTheme(s.theme) }"
    />

    <div v-if="gitOpen && gitProject" class="modal git-modal" @mousedown.self="gitOpen = false">
      <section class="dialog git-dialog">
        <header>
          <h2>{{ t('git.title', { name: gitProject.name }) }}</h2>
          <button type="button" @click="gitOpen = false"><X /></button>
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
