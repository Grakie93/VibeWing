<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import type { Chat, Settings } from '../types'

const props = defineProps<{ settings: Settings }>()
const chats = ref<Chat[]>([])
const activeId = ref('')
const input = ref('')
const busy = ref(false)
const elapsed = ref<number | null>(null)
const selectedModel = ref('')

const activeChat = computed(() => chats.value.find(chat => chat.id === activeId.value) || null)
const models = computed(() => props.settings.providers.flatMap(provider => {
  const ids = provider.models?.length ? provider.models : [provider.model]
  return ids.filter(Boolean).map(id => ({ id: `${provider.id}::${id}`, modelId: id, provider: provider.id, label: provider.model_names?.[id] || id }))
}))
const activeModel = computed(() => models.value.find(model => model.id === selectedModel.value) || models.value[0])

function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)) }
async function persist() { await desktop.saveChats(copy(chats.value)).catch(() => undefined) }
function createChat() {
  const now = Date.now()
  const chat: Chat = { id: String(now), title: '新对话', model: selectedModel.value, messages: [], updated_at: now }
  chats.value.unshift(chat); activeId.value = chat.id; void persist()
}
function ensureChat() { if (!activeChat.value) createChat(); return chats.value.find(chat => chat.id === activeId.value)! }
async function send() {
  const content = input.value.trim()
  if (!content || busy.value || !activeModel.value) return
  const chat = ensureChat(); input.value = ''; chat.model = activeModel.value.id
  chat.messages.push({ role: 'user', content }); if (chat.title === '新对话') chat.title = content.slice(0, 28)
  busy.value = true; elapsed.value = null; const started = performance.now()
  try {
    const result = await desktop.askAi({ provider_id: activeModel.value.provider, model: activeModel.value.modelId, messages: chat.messages })
    chat.messages.push({ role: 'assistant', content: result.content }); elapsed.value = result.elapsed_ms || Math.round(performance.now() - started)
  } catch (error) { chat.messages.push({ role: 'assistant', content: `请求失败：${String(error)}` }) }
  finally { chat.updated_at = Date.now(); busy.value = false; void persist() }
}
function keydown(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) return
  if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void send() }
}
watch(selectedModel, model => { if (model && activeChat.value) activeChat.value.model = model })
onMounted(async () => { chats.value = await desktop.listChats().catch(() => []); if (chats.value[0]) activeId.value = chats.value[0].id; selectedModel.value = props.settings.default_chat_model || models.value[0]?.id || '' })
</script>

<template>
  <section class="chat-panel">
    <div class="chat-layout">
      <aside class="chat-sidebar">
        <button type="button" class="new-chat" @click="createChat">＋ 新对话</button>
        <button v-for="chat in chats" :key="chat.id" type="button" :class="['chat-tab', { active: activeId === chat.id }]" @click="activeId = chat.id">{{ chat.title }}</button>
      </aside>
      <section class="chat-main">
        <div class="chat-messages">
          <template v-if="activeChat">
            <article v-for="(message, index) in activeChat.messages" :key="index" :class="['chat-message', message.role]">
              <span class="message-author">{{ message.role === 'user' ? '我' : 'AI' }}</span>
              <div class="message-bubble"><p>{{ message.content }}</p></div>
            </article>
          </template>
          <p v-if="busy" class="thinking">思考中…</p>
          <p v-if="!models.length" class="hint">请先在设置中添加模型平台并配置 API Key。</p>
        </div>
        <form class="chat-composer" @submit.prevent="send">
          <button type="button" class="attach-button" title="附加项目上下文">＋</button>
          <textarea v-model="input" rows="2" placeholder="输入问题，Enter 发送；Shift+Enter 换行" @keydown="keydown" />
          <div class="composer-actions">
            <select v-model="selectedModel" class="model-select"><option v-for="model in models" :key="model.provider + model.id" :value="model.id">{{ model.label }}</option></select>
            <button class="send-button primary" type="submit" :disabled="busy || !activeModel">➤</button>
          </div>
        </form>
      </section>
    </div>
    <small v-if="elapsed" class="elapsed">本次响应 {{ elapsed }} ms</small>
  </section>
</template>
