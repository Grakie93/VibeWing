<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { renderMarkdown } from '../services/markdown'
import { t } from '../i18n'
import type { Chat, ChatMessage, ProjectView, Settings } from '../types'

const props = defineProps<{ settings: Settings; projects?: ProjectView[]; pendingLog?: string }>()
const emit = defineEmits<{ memoryUpdated: [memory: string] }>()
const chats = ref<Chat[]>([])
const activeId = ref('')
const input = ref('')
const selectedModel = ref('')
const attachedLog = ref('')
const attachedProject = ref<ProjectView | null>(null)
const pinDialogOpen = ref(false)
const pinEditText = ref('')
const contextOpen = ref(false)
const expandedMessages = ref<Record<number, boolean>>({})
const logPreviewOpen = ref(false)
const pendingChats = ref<Record<string, boolean>>({})
const thinkingStarts = ref<Record<string, number>>({})
const now = ref(Date.now())

let thinkingTimer: ReturnType<typeof setInterval> | null = null

const messagesRef = ref<HTMLElement | null>(null)

const activeChat = computed(() => chats.value.find(chat => chat.id === activeId.value) || null)
const models = computed(() =>
  props.settings.providers.flatMap(provider => {
    const ids = provider.models?.length ? provider.models : [provider.model]
    return ids.filter(Boolean).map(id => ({
      id: `${provider.id}::${id}`,
      modelId: id,
      provider: provider.id,
      label: provider.model_names?.[id] || id,
    }))
  }),
)
const activeModel = computed(
  () => models.value.find(model => model.id === selectedModel.value) || models.value[0],
)
const busy = computed(() => Boolean(activeChat.value && pendingChats.value[activeChat.value.id]))
const thinkingSeconds = computed(() => {
  const chat = activeChat.value
  if (!chat || !pendingChats.value[chat.id]) return '0.0'
  const start = thinkingStarts.value[chat.id] || now.value
  return ((now.value - start) / 1000).toFixed(1)
})

function copy<T>(value: T): T {
  return JSON.parse(JSON.stringify(value))
}
async function persist() {
  await desktop.saveChats(copy(chats.value)).catch(() => undefined)
}

function createChat() {
  const now = Date.now()
  const chat: Chat = {
    id: String(now),
    title: t('chat.title'),
    model: selectedModel.value,
    messages: [],
    updated_at: now,
  }
  chats.value.unshift(chat)
  activeId.value = chat.id
  void persist()
}

function ensureChat() {
  if (!activeChat.value) createChat()
  return chats.value.find(chat => chat.id === activeId.value)!
}

function removeChat(id: string) {
  if (!id) return
  chats.value = chats.value.filter(chat => chat.id !== id)
  if (activeId.value === id) {
    activeId.value = chats.value[0]?.id || ''
  }
  void persist()
}

function openPinMemory(text: string) {
  pinEditText.value = text
  pinDialogOpen.value = true
}

async function confirmPinMemory() {
  const text = pinEditText.value.trim()
  if (!text) return
  const current = props.settings.memory || ''
  const next = current ? `${current}\n${text}` : text
  emit('memoryUpdated', next)
  pinDialogOpen.value = false
}

function startThinkingFor(chatId: string) {
  pendingChats.value[chatId] = true
  thinkingStarts.value[chatId] = Date.now()
  if (!thinkingTimer) {
    thinkingTimer = setInterval(() => {
      now.value = Date.now()
    }, 100)
  }
}

function stopThinkingFor(chatId: string) {
  pendingChats.value[chatId] = false
  delete thinkingStarts.value[chatId]
  if (!Object.keys(pendingChats.value).some(id => pendingChats.value[id])) {
    if (thinkingTimer) {
      clearInterval(thinkingTimer)
      thinkingTimer = null
    }
  }
}

async function send() {
  const question = input.value.trim()
  if (!question || busy.value || !activeModel.value) return
  const attachment = attachedLog.value
  const projectContext = attachedProject.value
  const chat = ensureChat()
  input.value = ''
  chat.model = activeModel.value.id

  const userMessage: ChatMessage = { role: 'user', content: question }
  if (attachment) userMessage.attachment = attachment
  chat.messages.push(userMessage)
  if (chat.title === t('chat.title')) chat.title = question.slice(0, 28)

  await nextTick()
  scrollToBottom()

  startThinkingFor(chat.id)
  const started = performance.now()
  try {
    const context = [
      projectContext
        ? t('chat.prompt.context', {
            name: projectContext.name,
            path: projectContext.path,
            frontend: projectContext.frontend_cmd || '-',
            backend: projectContext.backend_cmd || '-',
            frontendPort: projectContext.frontend_port || '-',
            backendPort: projectContext.backend_port || '-',
          })
        : '',
      attachment ? t('chat.prompt.attachment', { log: attachment }) : '',
    ]
      .filter(Boolean)
      .join('\n\n')
    const requestMessages = chat.messages.map((item, index) =>
      index === chat.messages.length - 1 && context ? { ...item, content: `${item.content}\n\n${context}` } : item,
    )
    const result = await desktop.askAi({
      provider_id: activeModel.value.provider,
      model: activeModel.value.modelId,
      messages: requestMessages,
    })
    const elapsed = result.elapsed_ms || Math.round(performance.now() - started)
    chat.messages.push({ role: 'assistant', content: result.content, elapsed_ms: elapsed })
  } catch (error) {
    chat.messages.push({ role: 'assistant', content: `${t('chat.requestFailed')}${String(error)}` })
  } finally {
    attachedLog.value = ''
    attachedProject.value = null
    logPreviewOpen.value = false
    chat.updated_at = Date.now()
    stopThinkingFor(chat.id)
    void persist()
    await nextTick()
    scrollToBottom()
  }
}

function scrollToBottom() {
  const node = messagesRef.value
  if (!node) return
  requestAnimationFrame(() => {
    node.scrollTop = node.scrollHeight
  })
}

function keydown(event: KeyboardEvent) {
  if (event.isComposing || event.keyCode === 229) return
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault()
    void send()
  }
}

function toggleAttachment(index: number) {
  expandedMessages.value[index] = !expandedMessages.value[index]
}

function formatElapsed(ms?: number) {
  if (ms === undefined || ms === null) return ''
  return `${(ms / 1000).toFixed(1)}s`
}

watch(selectedModel, model => {
  if (model && activeChat.value) activeChat.value.model = model
})
watch(
  () => props.pendingLog,
  log => {
    if (log) attachedLog.value = log.slice(0, 5000)
  },
)

onMounted(async () => {
  chats.value = await desktop.listChats().catch(() => [])
  if (chats.value[0]) activeId.value = chats.value[0].id
  selectedModel.value = props.settings.default_chat_model || models.value[0]?.id || ''
  attachedLog.value = props.pendingLog?.slice(0, 5000) || ''
})

onBeforeUnmount(() => {
  if (thinkingTimer) {
    clearInterval(thinkingTimer)
    thinkingTimer = null
  }
})

watch([activeId, () => activeChat.value?.messages.length, busy], () => {
  nextTick(scrollToBottom)
})
</script>

<template>
  <section class="chat-panel">
    <div class="chat-layout">
      <aside class="chat-sidebar">
        <button type="button" class="new-chat" @click="createChat">＋ {{ t('chat.new') }}</button>
        <div
          v-for="chat in chats"
          :key="chat.id"
          :class="['chat-tab-row', { active: activeId === chat.id }]"
        >
          <button
            type="button"
            class="chat-tab"
            @click="activeId = chat.id"
          >
            {{ chat.title || t('chat.title') }}
          </button>
          <button
            type="button"
            class="chat-tab-remove"
            :title="t('common.cancel')"
            @click.stop="removeChat(chat.id)"
          >×</button>
        </div>
        <p v-if="!chats.length" class="chat-sidebar-empty"></p>
      </aside>
      <section class="chat-main">
        <div ref="messagesRef" class="chat-messages">
          <template v-if="activeChat">
            <article
              v-for="(message, index) in activeChat.messages"
              :key="index"
              :class="['chat-message', message.role]"
            >
              <span class="message-author">
                <button
                  v-if="message.role === 'user'"
                  type="button"
                  class="pin-memory-btn"
                  :title="t('chat.pin.memory')"
                  :aria-label="t('chat.pin.memory')"
                  @click="openPinMemory(message.content)"
                >📌</button>
                <template v-if="message.role === 'user'">{{ t('chat.role.user') }}</template>
                <template v-else>
                  {{ t('chat.role.ai') }}
                  <template v-if="formatElapsed(message.elapsed_ms)"> · {{ formatElapsed(message.elapsed_ms) }}</template>
                </template>
              </span>
              <div class="message-bubble">
                <div v-if="message.role === 'assistant'" class="message-body" v-html="renderMarkdown(message.content)" />
                <template v-else>
                  <p>{{ message.content }}</p>
                  <div v-if="message.attachment" class="message-attachment">
                    <button
                      type="button"
                      class="attachment-toggle"
                      @click="toggleAttachment(index)"
                    >
                      {{ expandedMessages[index] ? t('chat.attach.collapse') : t('chat.attach.view') }}
                    </button>
                    <pre v-if="expandedMessages[index]" class="attachment-content">{{ message.attachment }}</pre>
                  </div>
                </template>
              </div>
            </article>
          </template>
          <article v-if="busy" class="chat-message assistant thinking-message">
            <span class="message-author">{{ t('chat.role.ai') }} · {{ t('chat.thinking') }} · {{ thinkingSeconds }}s</span>
            <div class="message-bubble thinking-bubble">
              <span class="thinking-dot" /><span class="thinking-dot" /><span class="thinking-dot" />
            </div>
          </article>
          <p v-if="!models.length" class="hint">{{ t('chat.noModels') }}</p>
        </div>
        <div class="chat-composer">
          <div v-if="attachedProject" class="chat-attachment">
            <span>{{ t('chat.attach.project') }} {{ attachedProject.name }}</span>
            <button type="button" :title="t('common.cancel')" @click.stop="attachedProject = null">×</button>
          </div>
          <pre v-if="attachedLog && logPreviewOpen" class="composer-attachment-preview">{{ attachedLog }}</pre>
          <div
            v-if="attachedLog"
            class="chat-attachment log-attachment-row"
            @click="logPreviewOpen = !logPreviewOpen"
          >
            <span class="log-attachment-arrow" :class="{ open: logPreviewOpen }">›</span>
            <span>{{ t('chat.attach.log') }} {{ attachedLog.length }} {{ t('chat.attach.unit') }}</span>
            <button type="button" :title="t('common.cancel')" @click.stop="attachedLog = ''">×</button>
          </div>
          <div class="composer-input-area">
            <button
              type="button"
              class="composer-icon attach-button"
              :title="t('chat.attach.title')"
              @click="contextOpen = !contextOpen"
            >＋</button>
            <div v-if="contextOpen" class="context-menu">
              <button
                v-for="project in (projects || [])"
                :key="project.id"
                type="button"
                @click="attachedProject = project; contextOpen = false"
              >{{ project.name }}</button>
              <span v-if="!(projects || []).length" class="hint">{{ t('chat.attach.empty') }}</span>
            </div>
            <textarea
              v-model="input"
              rows="4"
              :placeholder="t('chat.placeholder')"
              @keydown="keydown"
            />
            <div class="composer-toolbar">
              <select v-model="selectedModel" class="model-select">
                <option v-for="model in models" :key="model.provider + model.id" :value="model.id">
                  {{ model.label }}
                </option>
              </select>
              <button
                class="send-button primary"
                type="button"
                :disabled="busy || !activeModel"
                @click="send"
              >➤</button>
            </div>
          </div>
        </div>
      </section>
    </div>
    <div
      v-if="pinDialogOpen"
      class="modal pin-memory-modal"
      @mousedown.self="pinDialogOpen = false"
    >
      <section class="dialog pin-memory-dialog">
        <header>
          <h2>{{ t('chat.pin.memory.dialogTitle') }}</h2>
          <button
            type="button"
            :title="t('chat.pin.memory.cancel')"
            @click="pinDialogOpen = false"
          >×</button>
        </header>
        <p class="confirm-message">{{ t('chat.pin.memory.dialogDesc') }}</p>
        <textarea
          v-model="pinEditText"
          class="pin-memory-textarea"
          rows="5"
        />
        <footer>
          <span></span>
          <button type="button" @click="pinDialogOpen = false">{{ t('chat.pin.memory.cancel') }}</button>
          <button
            type="button"
            class="primary"
            :disabled="!pinEditText.trim()"
            @click="confirmPinMemory"
          >{{ t('chat.pin.memory.save') }}</button>
        </footer>
      </section>
    </div>
  </section>
</template>
