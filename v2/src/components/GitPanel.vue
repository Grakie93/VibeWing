<script setup lang="ts">
import { ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import type { ProjectView, Settings } from '../types'

interface GitFile { path: string; status: string; staged: boolean; unstaged: boolean }

const props = defineProps<{ project: ProjectView; settings?: Settings }>()
const files = ref<GitFile[]>([])
const scope = ref<'frontend' | 'backend'>('frontend')
const branches = ref<string[]>([])
const branch = ref('')
const selected = ref<string[]>([])
const message = ref('')
const busy = ref(false)
const error = ref('')
const aiBusy = ref(false)

function statusCode(file: GitFile) { return file.status.trim() || '?' }
function statusLabel(file: GitFile) {
  const code = statusCode(file)
  if (code.includes('??')) return '未跟踪'
  if (code.includes('A')) return '新增'
  if (code.includes('D')) return '删除'
  if (code.includes('R')) return '重命名'
  if (code.includes('M')) return '修改'
  return '变更'
}
function statusClass(file: GitFile) { return `git-status-${statusLabel(file)}` }

async function refresh() {
  error.value = ''
  try {
    files.value = await desktop.gitStatus(props.project.id, scope.value)
    branches.value = await desktop.gitBranches(props.project.id, scope.value)
    branch.value = await desktop.gitCurrentBranch(props.project.id, scope.value)
    selected.value = files.value.filter(file => file.staged).map(file => file.path)
  } catch (cause) { error.value = String(cause); files.value = []; branches.value = [] }
}
async function switchBranch() { if (!branch.value) return; await run('切换分支失败', () => desktop.gitSwitchBranch(props.project.id, scope.value, branch.value)) }
async function pull() { await run('拉取失败', () => desktop.gitPull(props.project.id, scope.value)) }
async function stage() { if (!selected.value.length) return; await run('暂存失败', () => desktop.gitStage(props.project.id, scope.value, selected.value)) }
function selectAll() { selected.value = files.value.filter(file => !(file.staged && !file.unstaged)).map(file => file.path) }
function clearSelection() { selected.value = [] }
async function commit() { if (!message.value.trim()) { error.value = '请先填写提交信息'; return }; await run('提交失败', async () => { await desktop.gitCommit(props.project.id, scope.value, message.value.trim()); message.value = '' }) }
async function generateMessage() {
  const provider = props.settings?.providers?.[0]
  const model = provider?.models?.[0] || provider?.model
  if (!provider || !model) { error.value = '请先在设置中配置模型平台和模型'; return }
  aiBusy.value = true; error.value = ''
  try {
    const summary = files.value.map(file => `${statusLabel(file)} ${file.path}`).join('\n')
    const result = await desktop.askAi({ provider_id: provider.id, model, messages: [{ role: 'user', content: `请根据以下 Git 变更生成一条具体的 Conventional Commit 提交信息，只返回一行，不要解释。\n\n${summary}` }] })
    message.value = result.content.trim().split('\\n')[0].replace(/^`+|`+$/g, '')
  } catch (cause) { error.value = `AI 生成失败：${String(cause)}` }
  finally { aiBusy.value = false }
}
async function push() { await run('推送失败', () => desktop.gitPush(props.project.id, scope.value)) }
async function run(label: string, action: () => Promise<unknown>) { busy.value = true; error.value = ''; try { await action(); await refresh() } catch (cause) { error.value = `${label}：${String(cause)}` } finally { busy.value = false } }
watch(scope, refresh, { immediate: true })
</script>

<template>
  <section class="git-panel">
    <div class="git-toolbar">
      <label class="git-scope">范围<select v-model="scope"><option value="frontend">前端</option><option value="backend">后端</option></select></label>
      <label class="git-branch">分支<select v-model="branch" @change="switchBranch"><option v-for="name in branches" :key="name" :value="name">{{ name }}</option></select></label>
      <button type="button" :disabled="busy" @click="pull">拉取</button>
      <button type="button" :disabled="busy" @click="refresh">刷新</button>
    </div>
    <div class="git-legend"><span><i class="git-status-dot git-status-新增" />新增</span><span><i class="git-status-dot git-status-修改" />修改</span><span><i class="git-status-dot git-status-删除" />删除</span><span><i class="git-status-dot git-status-未跟踪" />未跟踪</span></div>
    <p v-if="error" class="git-error">{{ error }}</p>
    <div v-if="!files.length" class="git-empty">工作区干净，没有待处理文件</div>
    <div v-else class="git-files"><label v-for="file in files" :key="file.path" class="git-file"><input v-model="selected" type="checkbox" :value="file.path" :disabled="file.staged && !file.unstaged" /><span :class="['git-status', statusClass(file)]">{{ statusLabel(file) }}</span><code :title="file.path">{{ file.path }}</code></label></div>
    <div class="git-stage-row"><span>{{ selected.length }} 个文件已选择</span><div class="git-select-actions"><button type="button" :disabled="busy" @click="selectAll">全选</button><button type="button" :disabled="busy" @click="clearSelection">清空选择</button><button type="button" :disabled="busy || !selected.length" @click="stage">暂存所选</button></div></div>
    <label class="commit-message">提交信息<textarea v-model="message" rows="3" placeholder="例如：feat: improve project controls" /></label>
    <div class="git-actions"><button type="button" :disabled="busy || aiBusy" @click="generateMessage">{{ aiBusy ? 'AI 编写中…' : 'AI 生成提交信息' }}</button><button type="button" :disabled="busy || !message.trim()" @click="commit">{{ busy ? '处理中…' : '提交暂存' }}</button><button type="button" class="primary" :disabled="busy" @click="push">推送远端</button></div>
  </section>
</template>
