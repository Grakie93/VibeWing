import { computed, ref } from 'vue'

export const language = ref<'zh-CN' | 'en'>('zh-CN')

const messages = {
  'zh-CN': {
    subtitle: '让你的项目轻盈起飞', ai: 'AI 对话', settings: '设置', import: '导入项目',
    empty: '还没有导入项目', start: '启动', restart: '重启', stop: '停止', logs: '查看日志',
    edit: '编辑', running: '运行中', stopped: '已停止', frontend: '前端', backend: '后端',
  },
  en: {
    subtitle: 'Give your projects wings.', ai: 'AI Chat', settings: 'Settings', import: 'Import Project',
    empty: 'No projects imported yet', start: 'Start', restart: 'Restart', stop: 'Stop', logs: 'View Logs',
    edit: 'Edit', running: 'Running', stopped: 'Stopped', frontend: 'Frontend', backend: 'Backend',
  },
} as const

export const text = computed(() => messages[language.value])
