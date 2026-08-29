<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import logo from './assets/vibewing-logo.png'
import ProjectCard from './components/ProjectCard.vue'
import ProjectEditor from './components/ProjectEditor.vue'
import { language, text } from './i18n'
import { desktop } from './services/desktop'
import type { Project, ProjectView } from './types'

const projects = ref<ProjectView[]>([])
const editorOpen = ref(false)
const editing = ref<ProjectView | null>(null)
let refreshTimer: number | undefined

async function refresh() { if (!document.hidden) projects.value = await desktop.listProjects() }
function openEditor(project: ProjectView | null = null) { editing.value = project; editorOpen.value = true }
async function save(project: Project) { await desktop.saveProject(project); editorOpen.value = false; await refresh() }
async function remove(id: string) { if (confirm('移除项目配置？不会删除项目文件。')) { await desktop.deleteProject(id); editorOpen.value = false; await refresh() } }

onMounted(async () => {
  const settings = await desktop.getSettings(); language.value = settings.language
  await refresh(); refreshTimer = window.setInterval(refresh, 10_000)
})
onBeforeUnmount(() => clearInterval(refreshTimer))
</script>

<template>
  <main>
    <header class="topbar">
      <div class="brand"><img :src="logo" alt="VibeWing" /><div><h1>VibeWing</h1><p>{{ text.subtitle }}</p></div></div>
      <nav><button disabled>💬 {{ text.ai }}</button><button disabled>⚙ {{ text.settings }}</button><button class="primary" @click="openEditor()">＋ {{ text.import }}</button></nav>
    </header>
    <section v-if="projects.length" class="project-grid"><ProjectCard v-for="project in projects" :key="project.id" :project="project" @edit="openEditor" @changed="refresh" /></section>
    <div v-else class="empty">{{ text.empty }}</div>
    <ProjectEditor :open="editorOpen" :project="editing" @close="editorOpen = false" @save="save" @remove="remove" />
  </main>
</template>
