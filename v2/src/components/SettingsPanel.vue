<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { language } from '../i18n'
import type { Provider, Settings } from '../types'

const props = defineProps<{ open: boolean; settings: Settings }>()
const emit = defineEmits<{ close: []; saved: [settings: Settings] }>()
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value))
const form = reactive<Settings>(clone(props.settings))
const tab = ref<'models' | 'general'>('models')
const providerId = ref('')
const key = ref('')
const editingProvider = ref(false)
const newModelId = ref('')
const newModelName = ref('')

const current = computed(() => form.providers.find(provider => provider.id === providerId.value))
const configuredView = computed(() => Boolean(current.value && !editingProvider.value && (current.value.base_url || current.value.model || current.value.models?.length)))
const keyConfigured = computed(() => Boolean(current.value?.key_configured))

function sync() {
  Object.assign(form, clone(props.settings))
  providerId.value = form.providers.find(provider => provider.id === providerId.value)?.id || form.providers[0]?.id || ''
  editingProvider.value = false
  key.value = ''
  newModelId.value = ''
  newModelName.value = ''
}
watch(() => props.open, open => { if (open) sync() })

function selectProvider(id: string) {
  providerId.value = id
  editingProvider.value = false
  key.value = ''
}
function addProvider() {
  const provider: Provider = { id: `provider-${Date.now()}`, name: '', base_url: '', model: '', models: [], model_names: {}, key_configured: false }
  form.providers.push(provider)
  providerId.value = provider.id
  editingProvider.value = true
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
function removeModel(id: string) {
  const provider = current.value
  if (!provider) return
  provider.models = (provider.models || []).filter(model => model !== id)
  if (provider.model === id) provider.model = provider.models[0] || ''
}
async function removeProvider() {
  if (!current.value || !window.confirm('删除这个模型平台？')) return
  const removedId = providerId.value
  form.providers = form.providers.filter(provider => provider.id !== removedId)
  providerId.value = form.providers[0]?.id || ''
  editingProvider.value = false
  try {
    await desktop.saveSettings(clone(form))
    emit('saved', clone(form))
  } catch (error) {
    window.alert(`删除平台失败：${String(error)}`)
  }
}
async function save() {
  const provider = current.value
  if (provider && key.value.trim()) {
    await desktop.saveProviderKey(provider.id, key.value.trim())
    provider.key_configured = true
  }
  const saved = clone(form)
  await desktop.saveSettings(saved)
  language.value = saved.language
  emit('saved', saved)
  emit('close')
}
</script>

<template>
  <div v-if="open" class="modal">
    <form class="dialog settings-dialog" @submit.prevent="save">
      <header><h2>{{ form.language === 'en' ? 'Settings' : '设置' }}</h2><button type="button" @click="emit('close')">×</button></header>
      <div class="settings-layout">
        <aside>
          <button type="button" :class="{ active: tab === 'models' }" @click="tab = 'models'">模型服务</button>
          <button type="button" :class="{ active: tab === 'general' }" @click="tab = 'general'">常规设置</button>
        </aside>
        <section v-if="tab === 'general'">
          <label>软件语言<select v-model="form.language"><option value="zh-CN">简体中文</option><option value="en">English</option></select></label>
          <label>主题<select v-model="form.theme.preset"><option value="winglight">蝶翼浅色</option><option value="wingdark">蝶翼深色</option></select></label>
          <label class="check-row"><input v-model="form.check_updates" type="checkbox" /> 自动检查更新</label>
        </section>
        <section v-else>
          <div class="provider-tabs">
            <span v-for="provider in form.providers" :key="provider.id" class="provider-chip">
              <button type="button" class="provider-main" :class="{ active: providerId === provider.id }" @click="selectProvider(provider.id)">{{ provider.name || '未命名平台' }}</button><button type="button" class="chip-remove" title="删除平台" @click="selectProvider(provider.id); removeProvider()">×</button>
            </span>
            <button type="button" @click="addProvider">＋ 添加平台</button>
          </div>
          <template v-if="current">
            <template v-if="configuredView">
              <p class="provider-summary">{{ current.name }} · 已配置</p>
              <div class="model-list">
                <span v-for="id in (current.models || [])" :key="id" class="model-chip">
                  <button type="button" @click="current.model = id">{{ current.model_names?.[id] || id }}</button>
                  <button type="button" class="chip-remove" title="删除模型" @click="removeModel(id)">×</button>
                </span>
                <span v-if="!current.models?.length" class="hint">还没有添加模型</span>
              </div>
              <div class="add-model-row">
                <label>模型 ID<input v-model="newModelId" placeholder="例如 openai/gpt-oss-120b" @keyup.enter="addModel" /></label>
                <label>显示名称<input v-model="newModelName" placeholder="可选" @keyup.enter="addModel" /></label>
                <button type="button" @click="addModel">＋ 添加模型</button>
              </div>
              <button type="button" class="edit-provider-button" @click="editingProvider = true">编辑平台配置</button>
            </template>
            <template v-else>
              <label>平台名称<input v-model="current.name" placeholder="例如 NVIDIA、DeepSeek" /></label>
              <label>API 地址<input v-model="current.base_url" placeholder="https://api.example.com/v1" /></label>
              <label>API Key<input v-model="key" type="password" :placeholder="keyConfigured ? '留空保持原 Key' : '输入 API Key'" /></label>
              <p class="hint">先保存平台配置，再添加一个或多个模型。</p>
              <div class="provider-edit-actions">
                <button type="button" @click="editingProvider = false">返回模型列表</button>
                <button type="button" class="danger" @click="removeProvider">删除平台</button>
              </div>
            </template>
          </template>
          <p v-else class="hint">请先添加模型平台。</p>
        </section>
      </div>
      <footer><span></span><button type="button" @click="emit('close')">取消</button><button class="primary" type="submit">保存设置</button></footer>
    </form>
  </div>
</template>
