<script setup lang="ts">
import { notifyState, removeNotify } from '../services/notify'
import { X } from '@lucide/vue'
</script>

<template>
  <div class="notify-container" aria-live="polite" aria-atomic="true">
    <div
      v-for="toast in notifyState"
      :key="toast.id"
      :class="['notify-toast', toast.type]"
      role="status"
    >
      <span class="notify-message">{{ toast.message }}</span>
      <button
        type="button"
        class="notify-close"
        aria-label="Close"
        @click="removeNotify(toast.id)"
      >
        <X :size="14" />
      </button>
    </div>
  </div>
</template>
