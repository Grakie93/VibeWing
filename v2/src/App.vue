<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import logo from './assets/vibewing-logo.png'
import ProjectCard from './components/ProjectCard.vue'
import ProjectEditor from './components/ProjectEditor.vue'
import { language, text } from './i18n'
import { desktop } from './services/desktop'
import type { Project, ProjectView } from './types'
import ChatPanel from './components/ChatPanel.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import GitPanel from './components/GitPanel.vue'

const projects = ref<ProjectView[]>([])
const editorOpen = ref(false)
const editing = ref<ProjectView | null>(null)
const settings = ref<any>({ language:'zh-CN', theme:{accent:'#20bdb7',bg:'#f2fbfb',card:'#fff',preset:'winglight'}, check_updates:true, default_chat_model:'', providers:[] })
const chatOpen = ref(false); const settingsOpen = ref(false)
const gitOpen = ref(false); const gitProject = ref<ProjectView | null>(null)
let refreshTimer: number | undefined

async function refresh() { if (!document.hidden) projects.value = await desktop.listProjects() }
function openEditor(project: ProjectView | null = null) { editing.value = project; editorOpen.value = true }
async function save(project: Project) { try { await desktop.saveProject(project); editorOpen.value = false; editing.value = null; await refresh() } catch (error) { alert(`保存项目失败：${String(error)}`) } }
async function remove(id: string) { if (confirm('移除项目配置？不会删除项目文件。')) { await desktop.deleteProject(id); editorOpen.value = false; await refresh() } }

onMounted(async () => {
  settings.value = await desktop.getSettings(); language.value = settings.value.language
  await refresh(); refreshTimer = window.setInterval(refresh, 10_000)
})
onBeforeUnmount(() => clearInterval(refreshTimer))
</script>

<template>
  <main>
    <header class="topbar">
      <div class="brand"><img :src="logo" alt="VibeWing" /><div><h1>VibeWing</h1><p>{{ text.subtitle }}</p></div></div>
      <nav><button @click="chatOpen=true">💬 {{ text.ai }}</button><button @click="settingsOpen=true">⚙ {{ text.settings }}</button><button class="primary" @click="openEditor()">＋ {{ text.import }}</button></nav>
    </header>
    <section v-if="projects.length" class="project-grid"><ProjectCard v-for="project in projects" :key="project.id" :project="project" @edit="openEditor" @changed="refresh" @git="gitProject=$event;gitOpen=true" /></section>
    <div v-else class="empty">{{ text.empty }}</div>
    <ProjectEditor :open="editorOpen" :project="editing" @close="editorOpen = false" @save="save" @remove="remove" />
    <div v-if="chatOpen" class="modal"><section class="dialog chat-dialog"><header><h2>{{ text.ai }}</h2><button @click="chatOpen=false">×</button></header><ChatPanel :settings="settings" /></section></div>
    <SettingsPanel :open="settingsOpen" :settings="settings" @close="settingsOpen=false" @saved="settings=$event;language=$event.language" />
    <div v-if="gitOpen && gitProject" class="modal"><section class="dialog git-dialog"><header><h2>Git · {{gitProject.name}}</h2><button @click="gitOpen=false">×</button></header><GitPanel :project="gitProject" /></section></div>
  </main>
</template>
