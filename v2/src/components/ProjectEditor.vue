<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { Project } from '../types'
import { emptyProject } from '../types'

const props = defineProps<{ open: boolean; project?: Project | null }>()
const emit = defineEmits<{ close: []; save: [project: Project]; remove: [id: string] }>()
const form = reactive<Project>(emptyProject())
const clone = (value: Project): Project => JSON.parse(JSON.stringify(value))
function normalize(){ if (!form.frontend_path.trim()) form.frontend_path=form.path; if (!form.backend_path.trim()) form.backend_path=form.path }
watch(() => [props.open, props.project], () => Object.assign(form, props.project ? clone(props.project) : emptyProject()), { immediate: true })
const submit = () => { normalize(); emit('save', clone(form) as Project) }
</script>

<template>
  <div v-if="open" class="modal"><form class="dialog" @submit.prevent="submit">
    <header><h2>{{ form.id ? '编辑项目' : '导入项目' }}</h2><button type="button" @click="emit('close')">×</button></header>
    <label>项目名称<input v-model="form.name" required /></label>
    <label>项目主目录<input v-model="form.path" required /></label>
    <div class="form-grid">
      <label>前端目录<input v-model="form.frontend_path" /></label><label>前端端口<input v-model="form.frontend_port" /></label>
      <label class="wide">前端启动命令<input v-model="form.frontend_cmd" /></label>
      <label>后端目录<input v-model="form.backend_path" /></label><label>后端端口<input v-model="form.backend_port" /></label>
      <label class="wide">后端启动命令<input v-model="form.backend_cmd" /></label>
    </div>
    <footer><button v-if="form.id" type="button" class="danger" @click="emit('remove', form.id)">移除项目</button><span /><button type="button" @click="emit('close')">取消</button><button class="primary">保存项目</button></footer>
  </form></div>
</template>
