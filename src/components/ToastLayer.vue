<script setup lang="ts">
import { useToast } from '../composables/useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <Teleport to="body">
    <div class="toast-layer">
      <TransitionGroup name="toast">
        <div
          v-for="t in toasts"
          :key="t.id"
          class="toast-item"
          :class="t.type"
          @click="remove(t.id)"
        >
          <span class="toast-msg">{{ t.message }}</span>
          <button
            v-if="t.action?.label"
            class="toast-action"
            @click.stop="t.action!.onClick()"
          >{{ t.action.label }}</button>
          <button class="toast-close" @click.stop="remove(t.id)">
            <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-layer {
  position: fixed;
  top: 12px;
  right: 12px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 6px;
  pointer-events: none;
}
.toast-item {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg);
  border: 1px solid var(--border-color);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1), 0 1px 3px rgba(0, 0, 0, 0.04);
  font-size: 12px;
  font-weight: 500;
  color: var(--fg);
  cursor: default;
  max-width: 320px;
}
.toast-item.success { border-left: 3px solid var(--accent); }
.toast-item.error { border-left: 3px solid var(--danger); }
.toast-item.info { border-left: 3px solid var(--muted); }

.toast-msg { flex: 1; line-height: 1.4; }
.toast-action {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  padding: 0;
  line-height: 1;
}
.toast-action:hover { text-decoration: underline; }
.toast-close {
  border: none;
  background: none;
  color: var(--muted);
  cursor: pointer;
  padding: 2px;
  display: flex;
  align-items: center;
  opacity: 0.4;
  transition: opacity 0.15s;
}
.toast-close:hover { opacity: 1; }

/* 过渡 */
.toast-enter-active { transition: all 0.2s ease-out; }
.toast-leave-active { transition: all 0.15s ease-in; }
.toast-enter-from { opacity: 0; transform: translateX(12px); }
.toast-leave-to { opacity: 0; transform: translateX(12px); }
.toast-move { transition: transform 0.2s ease; }
</style>