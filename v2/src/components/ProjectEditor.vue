<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { t } from '../i18n'
import { confirmDialog } from '../services/confirm'
import { desktop } from '../services/desktop'
import { Folder, X } from '@lucide/vue'
import type { Project } from '../types'
import { emptyProject } from '../types'

const props = defineProps<{ open: boolean; project?: Project | null }>()
const emit = defineEmits<{ close: []; save: [project: Project]; remove: [id: string] }>()
const saving = ref(false)
const form = reactive<Project>(emptyProject())
const copy = (value: Project): Project => JSON.parse(JSON.stringify(value))

watch(
  () => props.open,
  open => {
    if (!open) return
    Object.assign(form, props.project ? copy(props.project) : emptyProject())
    saving.value = false
  },
  { immediate: true },
)

watch(
  () => props.project,
  project => {
    if (props.open) {
      Object.assign(form, project ? copy(project) : emptyProject())
      saving.value = false
    }
  },
)

async function pickPath(field: 'path' | 'frontend_path' | 'backend_path') {
  const dir = await desktop.pickDirectory()
  if (dir) form[field] = dir
}

function submit() {
  if (saving.value) return
  if (!form.name.trim() || !form.path.trim()) return
  saving.value = true
  const project = copy(form)
  if (!project.frontend_path.trim()) project.frontend_path = project.path
  if (!project.backend_path.trim()) project.backend_path = project.path
  emit('save', project)
  window.setTimeout(() => {
    saving.value = false
  }, 800)
}

async function remove() {
  if (!form.id || saving.value) return
  const ok = await confirmDialog({
    message: t('editor.confirm.remove'),
    danger: true,
  })
  if (!ok) return
  emit('remove', form.id)
}
</script>

<template>
  <div v-if="open" class="modal" @mousedown.self="emit('close')">
    <form class="dialog project-editor" @submit.prevent="submit">
      <header>
        <h2>{{ form.id ? t('editor.title.edit') : t('editor.title.new') }}</h2>
        <button type="button" :aria-label="t('common.cancel')" @click="emit('close')"><X /></button>
      </header>
      <p v-if="form.source === 'file'" class="editor-imported-hint">
        {{ t('editor.importedHint') }}
      </p>
      <label>
        {{ t('editor.field.name') }}
        <input v-model.trim="form.name" required />
      </label>
      <label>
        {{ t('editor.field.path') }}
        <span class="path-field">
          <input v-model.trim="form.path" required placeholder="/path/to/project" />
          <button type="button" class="folder-pick" :title="t('editor.pickFolder')" :aria-label="t('editor.pickFolder')" @click="pickPath('path')"><Folder /></button>
        </span>
      </label>
      <div class="form-grid">
        <label>
          {{ t('editor.field.frontendPath') }}
          <span class="path-field">
            <input v-model.trim="form.frontend_path" />
            <button type="button" class="folder-pick" :title="t('editor.pickFolder')" :aria-label="t('editor.pickFolder')" @click="pickPath('frontend_path')"><Folder /></button>
          </span>
        </label>
        <label>
          {{ t('editor.field.frontendPort') }}
          <input v-model.trim="form.frontend_port" inputmode="numeric" />
        </label>
        <label class="wide">
          {{ t('editor.field.frontendCmd') }}
          <input v-model="form.frontend_cmd" placeholder="npm run dev" />
        </label>
        <label class="wide">
          {{ t('editor.field.frontendBuild') }}
          <input v-model="form.frontend_build" placeholder="npm run build" />
        </label>
        <label class="wide">
          {{ t('editor.field.frontendTestBuild') }}
          <input v-model="form.frontend_test_build" placeholder="npm run build -- --mode test" />
        </label>
        <label>
          {{ t('editor.field.backendPath') }}
          <span class="path-field">
            <input v-model.trim="form.backend_path" />
            <button type="button" class="folder-pick" :title="t('editor.pickFolder')" :aria-label="t('editor.pickFolder')" @click="pickPath('backend_path')"><Folder /></button>
          </span>
        </label>
        <label>
          {{ t('editor.field.backendPort') }}
          <input v-model.trim="form.backend_port" inputmode="numeric" />
        </label>
        <label class="wide">
          {{ t('editor.field.backendCmd') }}
          <input v-model="form.backend_cmd" placeholder="npm run server" />
        </label>
        <label class="wide">
          {{ t('editor.field.backendBuild') }}
          <input v-model="form.backend_build" placeholder="npm run build" />
        </label>
        <label class="wide">
          {{ t('editor.field.backendTestBuild') }}
          <input v-model="form.backend_test_build" placeholder="npm run build -- --mode test" />
        </label>
      </div>
      <footer>
        <button
          v-if="form.id"
          type="button"
          class="danger"
          @click="remove"
        >{{ t('editor.action.remove') }}</button>
        <div class="editor-actions-right">
          <button type="button" @click="emit('close')">{{ t('editor.action.cancel') }}</button>
          <button class="primary" type="submit" :disabled="saving">
            {{ saving ? t('editor.action.saving') : t('editor.action.save') }}
          </button>
        </div>
      </footer>
    </form>
  </div>
</template>
