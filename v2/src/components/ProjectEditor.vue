<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { t } from '../i18n'
import { confirmDialog } from '../services/confirm'
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
        <button type="button" :title="t('common.cancel')" @click="emit('close')">×</button>
      </header>
      <label>
        {{ t('editor.field.name') }}
        <input v-model.trim="form.name" required />
      </label>
      <label>
        {{ t('editor.field.path') }}
        <input v-model.trim="form.path" required placeholder="/path/to/project" />
      </label>
      <div class="form-grid">
        <label>
          {{ t('editor.field.frontendPath') }}
          <input v-model.trim="form.frontend_path" />
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
          <input v-model.trim="form.backend_path" />
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
          :title="t('editor.action.remove')"
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
