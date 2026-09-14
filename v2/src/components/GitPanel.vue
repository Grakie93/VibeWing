<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import { notify } from '../services/notify'
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
// Keep a separate commit message per scope so switching frontend/backend
// preserves each side's draft instead of wiping it.
const messages = ref<Record<'frontend' | 'backend', string>>({ frontend: '', backend: '' })
const message = computed({
  get: () => messages.value[scope.value],
  set: (value: string) => { messages.value[scope.value] = value },
})
const busy = ref(false)
const error = ref('')
const info = ref('')
const aiBusy = ref(false)
const commitLocale = ref<'zh' | 'en'>('zh')

// A file can be both staged and unstaged (e.g. staged then edited again). To
// keep each path in exactly one box, the "both" case is shown only under
// unstaged so the pending change is what the user acts on.
const stagedFiles = computed(() => files.value.filter(file => file.staged && !file.unstaged))
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
  info.value = ''
  try {
    files.value = await desktop.gitStatus(props.project.id, scope.value)
    branches.value = await desktop.gitBranches(props.project.id, scope.value)
    branch.value = await desktop.gitCurrentBranch(props.project.id, scope.value)
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

async function stagePaths(paths: string[]) {
  if (!paths.length) return
  await run(t('git.action.stage'), () => desktop.gitStage(props.project.id, scope.value, paths))
}
async function unstagePaths(paths: string[]) {
  if (!paths.length) return
  await run(t('git.action.unstage'), () => desktop.gitUnstage(props.project.id, scope.value, paths))
}
function stageFile(path: string) {
  void stagePaths([path])
}
function unstageFile(path: string) {
  void unstagePaths([path])
}
function stageAll() {
  void stagePaths(unstagedFiles.value.map(file => file.path))
}
function unstageAll() {
  void unstagePaths(stagedFiles.value.map(file => file.path))
}

async function commit() {
  if (!message.value.trim()) {
    error.value = t('git.error.emptyMessage')
    return
  }
  busy.value = true
  error.value = ''
  info.value = ''
  try {
    await desktop.gitCommit(props.project.id, scope.value, message.value.trim())
    message.value = ''
    notify('success', `${t('git.commit.commit')}${t('common.success')}`)
  } catch (cause) {
    const raw = String(cause)
    error.value = raw.toLowerCase().includes('nothing to commit')
      ? t('git.error.nothingToCommit')
      : `${t('git.commit.commit')}: ${raw}`
    notify('error', error.value)
  } finally {
    busy.value = false
    await refresh()
  }
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
  // Lock the scope this generation was triggered for: if the user switches
  // tabs while the request is in flight, the result must land back in the
  // original scope's box, not the one currently displayed.
  const targetScope = scope.value
  const fileList = files.value
  try {
    // Prefer the staged diff (what would actually be committed); fall back to
    // the working-tree diff when nothing is staged yet.
    let diffText = (await desktop.gitDiff(props.project.id, targetScope, true)).trim()
    if (!diffText) {
      diffText = (await desktop.gitDiff(props.project.id, targetScope, false)).trim()
    }
    diffText = diffText.replace(/\t/g, '  ').slice(0, 14000)
    const summary = fileList.map(file => `${statusLabel(file)} ${file.path}`).join('\n')
    if (!fileList.length) {
      error.value = t('git.error.nothingToCommit')
      return
    }
    const localeHint = commitLocale.value === 'zh' ? '使用简体中文' : 'in English'
    const diffSection = diffText
      ? `Diff：\n${diffText}`
      : 'Diff：无（仅根据上面的文件列表生成，不要反问）'
    const result = await desktop.askAi({
      provider_id: provider.id,
      model,
      messages: [
        {
          role: 'system',
          content: '你是一名资深软件工程师，专门负责根据 Git 变更写 Conventional Commits 提交信息。无论提供的信息是否充分，都必须直接输出一条提交信息，不要反问用户，不要要求补充信息，不要解释。',
        },
        {
          role: 'user',
          content: `请根据下面的 Git 变更写一条 Conventional Commits 风格的提交信息。
要求：
- 聚焦具体的功能/行为改动，不要罗列文件名，也不要写"提交xx文件、忽略xx文件"这类空泛描述。
- 格式：<type>(<scope>): <简短中文描述>，type 取 feat/fix/refactor/docs/style/chore/test/perf 之一，scope 用模块或目录名。
- 若改动较复杂，在标题下补一行以 "-" 开头的 body 说明原因或影响。
- 即使 diff 为空或信息有限，也请仅根据文件列表推断最可能的改动意图，直接输出提交信息，绝对禁止提问。
- 只输出提交信息本身，不要包含任何解释或引号包裹。
${localeHint}

变更文件列表：
${summary}

${diffSection}`,
        },
      ],
    })
    messages.value[targetScope] = result.content.trim().replace(/^```[\s\S]*?\n?/, '').replace(/```$/g, '').trim()
  } catch (cause) {
    error.value = `${t('git.error.aiFailed')}：${String(cause)}`
  } finally {
    aiBusy.value = false
  }
}

async function push() {
  info.value = ''
  // When there are uncommitted changes, commit them first (staging any
  // unstaged ones) so that "push" actually ships new commits instead of
  // silently reporting "up to date". Staged files are already part of the
  // next commit, so we only need to stage whatever is still unstaged.
  const dirty = files.value.some(file => file.unstaged || file.staged)
  if (dirty) {
    if (!message.value.trim()) {
      error.value = t('git.error.emptyMessage')
      notify('error', error.value)
      return
    }
    busy.value = true
    error.value = ''
    try {
      if (unstagedFiles.value.length) {
        await desktop.gitStage(props.project.id, scope.value, unstagedFiles.value.map(file => file.path))
      }
      await desktop.gitCommit(props.project.id, scope.value, message.value.trim())
      message.value = ''
      notify('success', `${t('git.commit.commit')}${t('common.success')}`)
    } catch (cause) {
      const raw = String(cause)
      error.value = raw.toLowerCase().includes('nothing to commit')
        ? t('git.error.nothingToCommit')
        : `${t('git.commit.commit')}: ${raw}`
      notify('error', error.value)
      await refresh()
      busy.value = false
      return
    }
    busy.value = false
  }
  busy.value = true
  error.value = ''
  try {
    const result = await desktop.gitPush(props.project.id, scope.value)
    notify('success', `${t('git.commit.push')}${t('common.success')}`)
    info.value = result.trim() || t('git.push.upToDate')
  } catch (cause) {
    error.value = `${t('git.commit.push')}: ${String(cause)}`
    notify('error', error.value)
  } finally {
    busy.value = false
    await refresh()
  }
}

async function run(label: string, action: () => Promise<unknown>) {
  busy.value = true
  error.value = ''
  try {
    await action()
    await refresh()
    notify('success', `${label}${t('common.success')}`)
  } catch (cause) {
    error.value = `${label}: ${String(cause)}`
    notify('error', error.value)
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

    <div class="git-legend">
      <span><i class="git-status-dot status-added"></i>{{ t('git.legend.added') }}</span>
      <span><i class="git-status-dot status-modified"></i>{{ t('git.legend.modified') }}</span>
      <span><i class="git-status-dot status-deleted"></i>{{ t('git.legend.deleted') }}</span>
      <span><i class="git-status-dot status-untracked"></i>{{ t('git.legend.untracked') }}</span>
    </div>

    <p v-if="error" class="git-error">{{ error }}</p>
    <p v-if="info" class="git-info">{{ info }}</p>

    <div v-if="!files.length" class="git-empty">{{ t('git.empty') }}</div>
    <div v-else class="git-workspace">
      <div class="git-columns">
        <div class="git-column">
          <div class="git-column-head">
            <h4>{{ t('git.unstaged') }}</h4>
            <button
              type="button"
              class="git-head-btn"
              :disabled="busy || !unstagedFiles.length"
              @click="stageAll"
            >
              {{ t('git.action.stageAll') }}
            </button>
          </div>
          <div class="git-file-box">
            <div v-if="!unstagedFiles.length" class="git-section-empty">—</div>
            <div
              v-for="file in unstagedFiles"
              :key="`u-${file.path}`"
              class="git-file"
            >
              <button
                type="button"
                class="git-toggle-btn"
                :disabled="busy"
                :aria-label="t('git.action.stage')"
                @click="stageFile(file.path)"
              >+</button>
              <span :class="['git-status', statusClass(file)]">{{ statusLabel(file) }}</span>
              <code>{{ file.path }}</code>
            </div>
          </div>
        </div>

        <div class="git-column">
          <div class="git-column-head">
            <h4>{{ t('git.staged') }}</h4>
            <button
              type="button"
              class="git-head-btn"
              :disabled="busy || !stagedFiles.length"
              @click="unstageAll"
            >
              {{ t('git.action.unstageAll') }}
            </button>
          </div>
          <div class="git-file-box staged">
            <div v-if="!stagedFiles.length" class="git-section-empty">—</div>
            <div
              v-for="file in stagedFiles"
              :key="`s-${file.path}`"
              class="git-file staged"
            >
              <button
                type="button"
                class="git-toggle-btn"
                :disabled="busy"
                :aria-label="t('git.action.unstage')"
                @click="unstageFile(file.path)"
              >−</button>
              <span :class="['git-status', statusClass(file)]">{{ statusLabel(file) }}</span>
              <code>{{ file.path }}</code>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="commit-wrap">
      <label class="commit-message">
        {{ t('git.commit.message') }}
        <textarea v-model="message" rows="3" :placeholder="t('git.commit.placeholder')" />
      </label>
      <div class="commit-lang-toggle" role="group" aria-label="Commit message language">
        <span class="slider" :class="{ zh: commitLocale === 'zh' }"></span>
        <button
          type="button"
          :class="{ active: commitLocale === 'en' }"
          :aria-pressed="commitLocale === 'en'"
          @click="commitLocale = 'en'"
        >
          EN
        </button>
        <button
          type="button"
          :class="{ active: commitLocale === 'zh' }"
          :aria-pressed="commitLocale === 'zh'"
          @click="commitLocale = 'zh'"
        >
          中文
        </button>
      </div>
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
