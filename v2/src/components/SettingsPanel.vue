<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { language } from '../i18n'
import type { Provider, Settings } from '../types'
const props = defineProps<{ open: boolean; settings: Settings }>()
const emit = defineEmits<{ close: []; saved: [settings: Settings] }>()
const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value))
const form = reactive<Settings>(clone(props.settings))
const tab = ref<'models' | 'general'>('models'); const providerId = ref(''); const key = ref(''); const keyConfigured = ref(false); const editingProvider = ref(false)
const newModelId = ref(''); const newModelName = ref('')
const current = computed(() => form.providers.find(p => p.id === providerId.value))
const configuredView = computed(() => Boolean(current.value && !editingProvider.value && (keyConfigured.value || current.value.base_url || current.value.model || current.value.models?.length)))
function sync() { Object.assign(form, clone(props.settings)); providerId.value = form.providers.find(p => p.id === providerId.value)?.id || form.providers[0]?.id || ''; editingProvider.value = false; key.value = ''; keyConfigured.value = Boolean(current.value?.key_configured) }
watch(() => props.open, open => { if (open) sync() })
function selectProvider(id: string) { providerId.value = id; editingProvider.value = false; key.value = ''; keyConfigured.value = Boolean(form.providers.find(p => p.id === id)?.key_configured) }
function addProvider() { const p: Provider = { id: `provider-${Date.now()}`, name: '', base_url: '', model: '', models: [], model_names: {}, key_configured: false }; form.providers.push(p); providerId.value = p.id; keyConfigured.value = false; editingProvider.value = true; tab.value = 'models' }
function addModel() { const p = current.value; const id = newModelId.value.trim(); if (!p || !id) return; p.models ||= []; if (!p.models.includes(id)) p.models.push(id); p.model_names ||= {}; if (newModelName.value.trim()) p.model_names[id] = newModelName.value.trim(); if (!p.model) p.model = id; newModelId.value = ''; newModelName.value = '' }
function removeModel(id: string) { const p = current.value; if (!p) return; p.models = (p.models || []).filter(item => item !== id); if (p.model === id) p.model = p.models[0] || '' }
function removeProvider() { if (!current.value || !confirm('删除这个模型平台？')) return; form.providers = form.providers.filter(p => p.id !== providerId.value); providerId.value = form.providers[0]?.id || ''; keyConfigured.value = false }
async function save() { const p = current.value; if (p && key.value.trim()) { await desktop.saveProviderKey(p.id, key.value.trim()); p.key_configured = true } const saved = clone(form); await desktop.saveSettings(saved); language.value = saved.language; emit('saved', saved); emit('close') }
</script>
<template>
  <div v-if="open" class="modal"><form class="dialog settings-dialog" @submit.prevent="save"><header><h2>{{ form.language === 'en' ? 'Settings' : '设置' }}</h2><button type="button" @click="emit('close')">×</button></header>
    <div class="settings-layout"><aside><button type="button" :class="{active:tab==='models'}" @click="tab='models'">模型服务</button><button type="button" :class="{active:tab==='general'}" @click="tab='general'">常规设置</button></aside>
      <section v-if="tab==='general'"><label>软件语言<select v-model="form.language"><option value="zh-CN">简体中文</option><option value="en">English</option></select></label><label>主题<select v-model="form.theme.preset"><option value="winglight">蝶翼浅色</option><option value="wingdark">蝶翼深色</option></select></label><label class="check-row"><input v-model="form.check_updates" type="checkbox"/> 自动检查更新</label></section>
      <section v-else><div class="provider-tabs"><span v-for="p in form.providers" :key="p.id" class="provider-chip"><button type="button" :class="{active:providerId===p.id}" @click="selectProvider(p.id)">{{p.name||'未命名平台'}}<span class="chip-remove" title="删除平台" @click.stop="selectProvider(p.id);removeProvider()">×</span></button></span><button type="button" @click="addProvider">＋ 添加平台</button></div>
        <template v-if="current"><template v-if="configuredView"><p class="provider-summary">{{current.name}} · 已配置</p><div class="model-list"><span v-for="id in (current.models?.length?current.models:[current.model])" :key="id" class="model-chip"><button type="button" @click="current.model=id">{{current.model_names?.[id]||id}}</button><button type="button" class="chip-remove" @click="removeModel(id)">×</button></span></div><div class="add-model-row"><input v-model="newModelId" placeholder="模型 ID" @keyup.enter="addModel"/><input v-model="newModelName" placeholder="显示名称（可选）" @keyup.enter="addModel"/><button type="button" @click="addModel">＋ 添加模型</button></div><button type="button" @click="editingProvider=true">编辑平台配置</button></template><template v-else><label>平台 ID<input v-model="current.id" /></label><label>平台名称<input v-model="current.name" /></label><label>API 地址<input v-model="current.base_url" /></label><label>API Key<input v-model="key" type="password" :placeholder="keyConfigured?'留空保持原 Key':'输入 API Key'"/></label><label>默认模型 ID<input v-model="current.model" /></label><label>模型名称<input :value="current.model_names?.[current.model]||''" @input="current.model_names![current.model]=($event.target as HTMLInputElement).value" /></label><div class="add-model-row"><input v-model="newModelId" placeholder="模型 ID" @keyup.enter="addModel"/><input v-model="newModelName" placeholder="显示名称（可选）" @keyup.enter="addModel"/><button type="button" @click="addModel">＋ 添加模型</button></div><div class="provider-edit-actions"><button type="button" @click="editingProvider=false">返回模型列表</button><button type="button" class="danger" @click="removeProvider">删除平台</button></div></template></template><p v-else class="hint">请先添加模型平台。</p>
      </section>
    </div><footer><span></span><button type="button" @click="emit('close')">取消</button><button class="primary" type="submit">保存设置</button></footer>
  </form></div>
</template>
