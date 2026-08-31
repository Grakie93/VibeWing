<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { Project } from '../types'
import { emptyProject } from '../types'

const props = defineProps<{ open: boolean; project?: Project | null }>()
const emit = defineEmits<{ close: []; save: [project: Project]; remove: [id: string] }>()
const saving = reactive({ value: false })
const form = reactive<Project>(emptyProject())
const copy = (value: Project): Project => JSON.parse(JSON.stringify(value))

watch(() => props.open, (open) => {
  if (!open) return
  Object.assign(form, props.project ? copy(props.project) : emptyProject())
}, { immediate: true })

function submit() {
  if (saving.value) return
  saving.value = true
  const project = copy(form)
  if (!project.frontend_path.trim()) project.frontend_path = project.path
  if (!project.backend_path.trim()) project.backend_path = project.path
  emit('save', project)
  window.setTimeout(() => { saving.value = false }, 800)
}
</script>

<template>
  <div v-if="open" class="modal">
    <form class="dialog project-editor" @submit.prevent="submit">
      <header><h2>{{ form.id ? '编辑项目' : '导入项目' }}</h2><button type="button" @click="emit('close')">×</button></header>
      <label>项目名称<input v-model.trim="form.name" required /></label>
      <label>项目主目录<input v-model.trim="form.path" required placeholder="/path/to/project" /></label>
      <div class="form-grid">
        <label>前端目录<input v-model.trim="form.frontend_path" /></label>
        <label>前端端口<input v-model.trim="form.frontend_port" inputmode="numeric" /></label>
        <label class="wide">前端启动命令<input v-model="form.frontend_cmd" placeholder="npm run dev" /></label>
        <label>后端目录<input v-model.trim="form.backend_path" /></label>
        <label>后端端口<input v-model.trim="form.backend_port" inputmode="numeric" /></label>
        <label class="wide">后端启动命令<input v-model="form.backend_cmd" placeholder="npm run server" /></label>
      </div>
      <footer>
        <button v-if="form.id" type="button" class="danger" @click.stop.prevent="emit('remove', form.id)">移除项目</button>
        <span />
        <button type="button" @click="emit('close')">取消</button>
        <button class="primary" type="submit" :disabled="saving.value">{{ saving.value ? '保存中…' : '保存项目' }}</button>
      </footer>
    </form>
  </div>
</template>
