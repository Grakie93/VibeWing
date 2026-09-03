<script setup lang="ts">
import { language, t } from '../i18n'

interface Props {
  open: boolean
  title: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}
const props = withDefaults(defineProps<Props>(), {
  confirmText: () => '',
  cancelText: () => '',
  danger: false,
})
const emit = defineEmits<{ confirm: []; cancel: [] }>()

const confirmLabel = () => props.confirmText || t('common.confirm')
const cancelLabel = () => props.cancelText || t('common.cancel')
</script>

<template>
  <div v-if="open" class="modal" @mousedown.self="emit('cancel')">
    <section class="dialog confirm-dialog" :dir="language === 'en' ? 'ltr' : 'ltr'">
      <header><h2>{{ title }}</h2></header>
      <p class="confirm-message">{{ message }}</p>
      <footer>
        <span></span>
        <button type="button" @click="emit('cancel')">{{ cancelLabel() }}</button>
        <button type="button" :class="danger ? 'danger-solid' : 'primary'" @click="emit('confirm')">{{ confirmLabel() }}</button>
      </footer>
    </section>
  </div>
</template>
