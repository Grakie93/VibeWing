<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { t } from '../i18n'
import type { ProjectView, Settings } from '../types'

interface GitFile {
  path: string
  status: string
  staged: boolean
  unstaged: boolean
}

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
const commitLocale = ref<'zh' | 'en'>('zh')

const stagedFiles = computed(() => files.value.filter(file => file.staged))
const unstagedFiles = computed(() => files.value.filter(file => file.unstaged))

function statusCode(file: GitFile) {
  return file.status.trim() || t('git.status.unknown')
}

function statusKey(file: GitFile): string {
  const code = statusCode(file)
  if (code.includes('??')) return 'git.status.untracked'
  if (code.includes('A')) return 'git.status.added'
  if (code.includes('D')) return 'git.status.deleted'
  if (code.includes('R')) return 'git.status.renamed'
  if (code.includes('M')) return 'git.status.modified'
  return 'git.status.changed'
}

function statusLabel(file: GitFile) {
  return t(statusKey(file))
}

function statusClass(file: GitFile) {
  const code = statusCode(file)
  if (code.includes('??')) return 'status-untracked'
  if (code.includes('A')) return 'status-added'
  if (code.includes('D')) return 'status-deleted'
  if (code.includes('R')) return 'status-renamed'
  if (code.includes('M')) return 'status-modified'
  return 'status-changed'
}

async function refresh() {
  error.value = ''
  try {
    files.value = await desktop.gitStatus(props.project.id, scope.value)
    branches.value = await desktop.gitBranches(props.project.id, scope.value)
    branch.value = await desktop.gitCurrentBranch(props.project.id, scope.value)
    selected.value = files.value.filter(file => !(file.staged && !file.unstaged)).map(file => file.path)
  } catch (cause) {
    error.value = String(cause)
    files.value = []
    branches.value = []
  }
}

async function switchBranch() {
  if (!branch.value) return
  await run(t('git.action.pull'), () => desktop.gitSwitchBranch(props.project.id, scope.value, branch.value))
}

async function pull() {
  await run(t('git.action.pull'), () => desktop.gitPull(props.project.id, scope.value))
}

async function stage() {
  if (!selected.value.length) return
  await run(t('git.action.stage'), () => desktop.gitStage(props.project.id, scope.value, selected.value))
}

function selectAll() {
  selected.value = files.value.filter(file => !(file.staged && !file.unstaged)).map(file => file.path)
}
function clearSelection() {
  selected.value = []
}

function toggleCommitLocale() {
  commitLocale.value = commitLocale.value === 'zh' ? 'en' : 'zh'
}

async function commit() {
  if (!message.value.trim()) {
    error.value = t('git.error.emptyMessage')
    return
  }
  await run(t('git.commit.commit'), async () => {
    await desktop.gitCommit(props.project.id, scope.value, message.value.trim())
    message.value = ''
  })
}

async function generateMessage() {
  const provider = props.settings?.providers?.[0]
  const model = provider?.models?.[0] || provider?.model
  if (!provider || !model) {
    error.value = t('git.error.noModel')
    return
  }
  aiBusy.value = true
  error.value = ''
  try {
    const summary = files.value.map(file => `${statusLabel(file)} ${file.path}`).join('\n')
    const result = await desktop.askAi({
      provider_id: provider.id,
      model,
      messages: [
        {
          role: 'user',
          content: `Please generate a specific Conventional Commit message based on the following Git changes. Return only one line. ${commitLocale.value === 'zh' ? 'Reply in Simplified Chinese.' : 'Reply in English.'}\n\n${summary}`,
        },
      ],
    })
    message.value = result.content.trim().split('\n')[0].replace(/^`+|`+$/g, '')
  } catch (cause) {
    error.value = `${t('git.error.aiFailed')}：${String(cause)}`
  } finally {
    aiBusy.value = false
  }
}

async function push() {
  await run(t('git.commit.push'), () => desktop.gitPush(props.project.id, scope.value))
}

async function run(label: string, action: () => Promise<unknown>) {
  busy.value = true
  error.value = ''
  try {
    await action()
    await refresh()
  } catch (cause) {
    error.value = `${label}: ${String(cause)}`
  } finally {
    busy.value = false
  }
}

watch(scope, refresh, { immediate: true })
</script>

<template>
  <section class="git-panel">
    <div class="git-toolbar">
      <label class="git-scope">
        {{ t('git.scope') }}
        <select v-model="scope">
          <option value="frontend">{{ t('service.frontend') }}</option>
          <option value="backend">{{ t('service.backend') }}</option>
        </select>
      </label>
      <label class="git-branch">
        {{ t('git.branch') }}
        <select v-model="branch" @change="switchBranch">
          <option v-for="name in branches" :key="name" :value="name">{{ name }}</option>
        </select>
      </label>
      <button type="button" :disabled="busy" @click="pull">{{ t('git.action.pull') }}</button>
      <button type="button" :disabled="busy" @click="refresh">{{ t('git.action.refresh') }}</button>
    </div>

    <div class="git-actions-top">
      <div class="git-select-actions">
        <button type="button" :disabled="busy" @click="selectAll">{{ t('git.action.selectAll') }}</button>
        <button type="button" :disabled="busy" @click="clearSelection">{{ t('git.action.clear') }}</button>
        <button type="button" :disabled="busy || !selected.length" @click="stage">
          {{ t('git.action.stage') }}
        </button>
      </div>
      <span class="git-selected-count">{{ t('git.selectedCount', { n: selected.length }) }}</span>
    </div>

    <div class="git-legend">
      <span><i class="git-status-dot status-added"></i>{{ t('git.legend.added') }}</span>
      <span><i class="git-status-dot status-modified"></i>{{ t('git.legend.modified') }}</span>
      <span><i class="git-status-dot status-deleted"></i>{{ t('git.legend.deleted') }}</span>
      <span><i class="git-status-dot status-untracked"></i>{{ t('git.legend.untracked') }}</span>
    </div>

    <p v-if="error" class="git-error">{{ error }}</p>

    <div v-if="!files.length" class="git-empty">{{ t('git.empty') }}</div>
    <div v-else class="git-files">
      <div class="git-file-section">
        <h4>{{ t('git.unstaged') }}</h4>
        <div v-if="!unstagedFiles.length" class="git-section-empty">—</div>
        <label
          v-for="file in unstagedFiles"
          :key="`u-${file.path}`"
          class="git-file"
        >
          <input v-model="selected" type="checkbox" :value="file.path" />
          <span :class="['git-status', statusClass(file)]">{{ statusLabel(file) }}</span>
          <code :title="file.path">{{ file.path }}</code>
        </label>
      </div>
      <div class="git-file-section">
        <h4>{{ t('git.staged') }}</h4>
        <div v-if="!stagedFiles.length" class="git-section-empty">—</div>
        <label
          v-for="file in stagedFiles"
          :key="`s-${file.path}`"
          class="git-file"
          :class="{ disabled: file.staged && !file.unstaged }"
        >
          <input
            v-model="selected"
            type="checkbox"
            :value="file.path"
            :disabled="file.staged && !file.unstaged"
          />
          <span :class="['git-status', statusClass(file)]">{{ statusLabel(file) }}</span>
          <code :title="file.path">{{ file.path }}</code>
        </label>
      </div>
    </div>

    <div class="commit-wrap">
      <label class="commit-message">
        {{ t('git.commit.message') }}
        <textarea v-model="message" rows="3" :placeholder="t('git.commit.placeholder')" />
      </label>
      <button type="button" class="commit-lang-toggle" @click="toggleCommitLocale">
        {{ commitLocale === 'zh' ? '中' : 'En' }}
      </button>
    </div>

    <div class="git-actions">
      <button type="button" :disabled="busy || aiBusy" @click="generateMessage">
        {{ aiBusy ? t('git.commit.aiBusy') : t('git.commit.ai') }}
      </button>
      <button type="button" :disabled="busy || !message.trim()" @click="commit">
        {{ busy ? t('git.commit.busy') : t('git.commit.commit') }}
      </button>
      <button type="button" class="primary" :disabled="busy" @click="push">{{ t('git.commit.push') }}</button>
    </div>
  </section>
</template>
