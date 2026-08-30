<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import type { Chat, Settings } from '../types'

const props = defineProps<{ settings: Settings }>()
const input = ref(''); const busy = ref(false); const elapsed = ref<number | null>(null)
const chats = ref<Chat[]>([]); const activeId = ref('')
const active = computed(() => chats.value.find(c => c.id === activeId.value) || null)
const providers = computed(() => props.settings.providers || [])
const selectedModel = ref('')
const modelOptions = computed(() => providers.value.flatMap(p => {
  const ids = p.models?.length ? p.models : [p.model]
  return ids.filter(Boolean).map(id => ({ id, label: p.model_names?.[id] || id, provider: p.id }))
}))
const selected = computed(() => modelOptions.value.find(m => m.id === selectedModel.value) || modelOptions.value[0])

function newChat() { const now=Date.now(); const chat:Chat={id:String(now),title:'新对话',model:selectedModel.value,messages:[],updated_at:now}; chats.value.unshift(chat); activeId.value=chat.id; persist() }
async function persist(){ try { await desktop.saveChats(chats.value) } catch {} }
function ensureChat(){ if(!active.value) newChat(); return active.value! }
async function send(){ const content=input.value.trim(); if(!content||busy.value||!selected.value)return; const chat=ensureChat(); input.value=''; chat.model=selected.value.id; chat.messages.push({role:'user',content}); if(chat.title==='新对话') chat.title=content.slice(0,32); busy.value=true; elapsed.value=null; const started=performance.now(); try { const result=await desktop.askAi({provider_id:selected.value.provider,model:selected.value.id,messages:chat.messages}); chat.messages.push({role:'assistant',content:result.content}); elapsed.value=result.elapsed_ms || Math.round(performance.now()-started) } catch(error) { chat.messages.push({role:'assistant',content:`请求失败：${String(error)}`}) } finally { chat.updated_at=Date.now(); busy.value=false; persist() } }
function onKeydown(e:KeyboardEvent){ if(e.isComposing||e.keyCode===229)return; if(e.key==='Enter'&&!e.shiftKey){e.preventDefault();send()} }
watch(selectedModel, value => { if(value && active.value) active.value.model=value })
onMounted(async()=>{ chats.value=await desktop.listChats().catch(()=>[]); if(chats.value.length) activeId.value=chats.value[0].id; selectedModel.value=props.settings.default_chat_model || modelOptions.value[0]?.id || '' })
</script>
<template>
  <section class="chat-panel">
    <div class="chat-layout"><aside class="chat-sidebar"><button type="button" class="new-chat" @click="newChat">＋ 新对话</button><button v-for="chat in chats" :key="chat.id" type="button" :class="['chat-tab',{active:chat.id===activeId}]" @click="activeId=chat.id">{{chat.title}}</button></aside><section class="chat-main"><div class="chat-messages"><template v-if="active"><article v-for="(message,index) in active.messages" :key="index" :class="['chat-message',message.role]"><strong>{{message.role==='user'?'我':'AI'}}</strong><p>{{message.content}}</p></article></template><p v-if="busy" class="thinking">思考中…</p><p v-if="!modelOptions.length" class="hint">请先在设置中添加模型平台并配置 API Key。</p></div><form class="chat-composer" @submit.prevent="send"><button type="button" class="attach-button">＋</button><textarea v-model="input" rows="2" placeholder="输入问题，Enter 发送；Shift+Enter 换行" @keydown="onKeydown"/><select v-model="selectedModel" class="model-select"><option v-for="model in modelOptions" :key="model.provider+model.id" :value="model.id">{{model.label}}</option></select><button class="send-button primary" :disabled="busy||!selected">➤</button></form></section></div>
    <small v-if="elapsed" class="elapsed">本次响应 {{elapsed}} ms</small>
  </section>
</template>
