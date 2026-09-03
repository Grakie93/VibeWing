<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { language, t } from '../i18n'
import { confirmDialog } from '../services/confirm'
import type { Provider, Settings } from '../types'

const props = defineProps<{ open: boolean; settings: Settings }>()
const emit = defineEmits<{ close: []; saved: [settings: Settings] }>()
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value))
const form = reactive<Settings>(clone(props.settings))
const tab = ref<'models' | 'general'>('models')
const providerId = ref('')
const key = ref('')
const editingProvider = ref(false)
const isNewProvider = ref(false)
const newModelId = ref('')
const newModelName = ref('')
const busy = ref(false)

const current = computed(() => form.providers.find(provider => provider.id === providerId.value))
const configuredView = computed(() =>
  Boolean(
    current.value &&
      !editingProvider.value &&
      (current.value.base_url || current.value.model || current.value.models?.length),
  ),
)
const keyConfigured = computed(() => Boolean(current.value?.key_configured))

function sync() {
  Object.assign(form, clone(props.settings))
  providerId.value =
    form.providers.find(provider => provider.id === providerId.value)?.id || form.providers[0]?.id || ''
  editingProvider.value = false
  isNewProvider.value = false
  key.value = ''
  newModelId.value = ''
  newModelName.value = ''
  busy.value = false
}
watch(() => props.open, open => {
  if (open) sync()
})

function selectProvider(id: string) {
  providerId.value = id
  editingProvider.value = false
  isNewProvider.value = false
  key.value = ''
}

function addProvider() {
  const provider: Provider = {
    id: `provider-${Date.now()}`,
    name: '',
    base_url: '',
    model: '',
    models: [],
    model_names: {},
    key_configured: false,
  }
  form.providers.push(provider)
  providerId.value = provider.id
  editingProvider.value = true
  isNewProvider.value = true
  tab.value = 'models'
}

function addModel() {
  const provider = current.value
  const id = newModelId.value.trim()
  if (!provider || !id) return
  provider.models ||= []
  if (!provider.models.includes(id)) provider.models.push(id)
  provider.model_names ||= {}
  if (newModelName.value.trim()) provider.model_names[id] = newModelName.value.trim()
  if (!provider.model) provider.model = id
  newModelId.value = ''
  newModelName.value = ''
}

async function removeModel(id: string) {
  const provider = current.value
  if (!provider) return
  const ok = await confirmDialog({
    message: t('settings.models.deleteModelConfirm'),
    danger: true,
  })
  if (!ok) return
  provider.models = (provider.models || []).filter(model => model !== id)
  if (provider.model === id) provider.model = provider.models[0] || ''
}

async function removeProvider(targetId?: string) {
  const id = targetId || providerId.value
  if (!id) return
  const provider = form.providers.find(p => p.id === id)
  if (!provider) return

  const ok = await confirmDialog({
    message: t('settings.models.deleteConfirm'),
    danger: true,
  })
  if (!ok) return

  const index = form.providers.findIndex(p => p.id === id)
  if (index < 0) return
  form.providers.splice(index, 1)
  if (providerId.value === id) {
    providerId.value = form.providers[0]?.id || ''
    editingProvider.value = false
  }

  busy.value = true
  try {
    const saved = clone(form)
    await desktop.saveSettings(saved)
    await desktop.saveProviderKey(id, '')
    emit('saved', saved)
  } catch (error) {
    alert(`${t('settings.models.deleteFailed')}：${String(error)}`)
  } finally {
    busy.value = false
  }
}

async function doSave(close: boolean) {
  if (busy.value) return
  busy.value = true
  try {
    const provider = current.value
    if (provider && key.value.trim()) {
      await desktop.saveProviderKey(provider.id, key.value.trim())
      provider.key_configured = true
    }
    const saved = clone(form)
    await desktop.saveSettings(saved)
    language.value = saved.language as 'zh-CN' | 'en'
    isNewProvider.value = false
    emit('saved', saved)
    if (close) emit('close')
  } finally {
    busy.value = false
  }
}

async function save() {
  await doSave(true)
}

async function confirmAdd() {
  await doSave(false)
  editingProvider.value = false
  key.value = ''
}
</script>

<template>
  <div v-if="open" class="modal" @mousedown.self="emit('close')">
    <form class="dialog settings-dialog" @submit.prevent="save">
      <header>
        <h2>{{ t('settings.title') }}</h2>
        <button type="button" :title="t('common.cancel')" @click="emit('close')">×</button>
      </header>
      <div class="settings-layout">
        <aside>
          <button type="button" :class="{ active: tab === 'models' }" @click="tab = 'models'">
            {{ t('settings.tab.models') }}
          </button>
          <button type="button" :class="{ active: tab === 'general' }" @click="tab = 'general'">
            {{ t('settings.tab.general') }}
          </button>
        </aside>

        <section v-if="tab === 'general'">
          <label>
            {{ t('settings.general.language') }}
            <select v-model="form.language">
              <option value="zh-CN">{{ t('settings.general.language.zh-CN') }}</option>
              <option value="en">{{ t('settings.general.language.en') }}</option>
            </select>
          </label>
          <label>
            {{ t('settings.general.theme') }}
            <select v-model="form.theme.preset">
              <option value="winglight">{{ t('settings.general.theme.winglight') }}</option>
              <option value="wingdark">{{ t('settings.general.theme.wingdark') }}</option>
            </select>
          </label>
          <label class="check-row">
            <input v-model="form.check_updates" type="checkbox" />
            {{ t('settings.general.checkUpdates') }}
          </label>
          <label>
            {{ t('settings.general.memory') }}
            <textarea
              v-model="form.memory"
              rows="4"
              :placeholder="t('settings.general.memory.placeholder')"
            />
            <span class="hint">{{ t('settings.general.memory.hint') }}</span>
          </label>
        </section>

        <section v-else>
          <div class="provider-tabs">
            <span
              v-for="provider in form.providers"
              :key="provider.id"
              :class="['provider-chip', { active: providerId === provider.id }]"
            >
              <button
                type="button"
                class="provider-main"
                @click="selectProvider(provider.id)"
              >{{ provider.name || t('settings.models.unnamed') }}</button>
              <button
                type="button"
                class="chip-remove"
                :title="t('settings.models.deletePlatform')"
                @click.stop="removeProvider(provider.id)"
              >×</button>
            </span>
            <button type="button" @click="addProvider">＋ {{ t('settings.models.add') }}</button>
          </div>

          <template v-if="current">
            <template v-if="configuredView">
              <p class="provider-summary">
                {{ current.name }} · {{ t('settings.models.summary') }}
                <button
                  type="button"
                  class="inline-edit"
                  :title="t('settings.models.editProvider')"
                  @click="editingProvider = true"
                >✎</button>
              </p>
              <div class="model-list">
                <span
                  v-for="id in (current.models || [])"
                  :key="id"
                  :class="['model-chip', { active: current.model === id }]"
                >
                  <button type="button" @click="current.model = id">
                    {{ current.model_names?.[id] || id }}
                  </button>
                  <button type="button" class="chip-remove" :title="t('common.cancel')" @click.stop="removeModel(id)">×</button>
                </span>
                <span v-if="!current.models?.length" class="hint">{{ t('settings.models.noModel') }}</span>
              </div>
              <div class="add-model-row">
                <label>
                  {{ t('settings.models.modelId') }}
                  <input
                    v-model="newModelId"
                    :placeholder="t('settings.models.modelId.placeholder')"
                    @keyup.enter="addModel"
                  />
                </label>
                <label>
                  {{ t('settings.models.modelName') }}
                  <input
                    v-model="newModelName"
                    :placeholder="t('settings.models.modelName.placeholder')"
                    @keyup.enter="addModel"
                  />
                </label>
                <button type="button" @click="addModel">＋ {{ t('settings.models.addModel') }}</button>
              </div>
            </template>
            <template v-else>
              <label>
                {{ t('settings.models.providerName') }}
                <input v-model="current.name" :placeholder="t('settings.models.providerName.placeholder')" />
              </label>
              <label>
                {{ t('settings.models.baseUrl') }}
                <input v-model="current.base_url" :placeholder="t('settings.models.baseUrl.placeholder')" />
              </label>
              <label>
                {{ t('settings.models.apiKey') }}
                <input
                  v-model="key"
                  type="password"
                  :placeholder="
                    keyConfigured
                      ? t('settings.models.apiKey.placeholder.keep')
                      : t('settings.models.apiKey.placeholder')
                  "
                />
              </label>
              <p class="hint">{{ t('settings.models.hint.firstSave') }}</p>
              <div class="provider-edit-actions">
                <button
                  v-if="isNewProvider"
                  type="button"
                  class="primary"
                  :disabled="busy || !current.name.trim() || !current.base_url.trim()"
                  @click="confirmAdd"
                >
                  {{ busy ? t('settings.action.saving') : t('settings.models.confirmAdd') }}
                </button>
                <button
                  v-else
                  type="button"
                  class="primary"
                  :disabled="busy"
                  @click="confirmAdd"
                >
                  {{ t('settings.action.save') }}
                </button>
              </div>
            </template>
          </template>
          <p v-else class="hint">{{ t('settings.models.empty') }}</p>
        </section>
      </div>
      <footer>
        <span></span>
        <button type="button" @click="emit('close')">{{ t('settings.action.cancel') }}</button>
        <button class="primary" type="submit" :disabled="busy">{{ t('settings.action.save') }}</button>
      </footer>
    </form>
  </div>
</template>
