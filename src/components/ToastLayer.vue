<script setup lang="ts">
import { useToast } from '../composables/useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <Teleport to="body">
    <!-- [m-20] 无障碍：每条自带语义（success/info → role="status" 即 polite；
         error/warning → role="alert" 即 assertive），这两个角色**本身就是 live region**，
         所以外层不再叠 aria-live —— 叠了会被读屏软件重复播报一遍。 -->
    <div class="toast-layer" role="region" aria-label="提示消息">
      <TransitionGroup name="toast">
        <div
          v-for="t in toasts"
          :key="t.id"
          class="toast-item"
          :class="t.type"
          :role="t.type === 'error' || t.type === 'warning' ? 'alert' : 'status'"
          @click="remove(t.id)"
        >
          <!-- 语义图标（四类）+ 左侧 3px 色条：不只靠颜色区分状态 -->
          <svg class="toast-icon" width="14" height="14" viewBox="0 0 24 24" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path v-if="t.type === 'success'" d="M20 6 9 17l-5-5"/>
            <template v-else-if="t.type === 'error'">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </template>
            <template v-else-if="t.type === 'warning'">
              <path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
              <path d="M12 9v4"/><path d="M12 17h.01"/>
            </template>
            <template v-else>
              <circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>
            </template>
          </svg>
          <span class="toast-msg">{{ t.message }}</span>
          <button
            v-if="t.action?.label"
            class="toast-action"
            @click.stop="t.action!.onClick()"
          >{{ t.action.label }}</button>
          <button class="toast-close" @click.stop="remove(t.id)" aria-label="关闭提示">
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
/* 位置：顶栏下方（52px），不再压住发送/关闭按钮（方案 M-4 / 4.8.5） */
.toast-layer {
  position: fixed;
  top: var(--toast-top);
  right: var(--toast-right);
  z-index: var(--z-toast);
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  pointer-events: none;
}
.toast-item {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-10);
  border-radius: var(--radius-md);
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-left: 3px solid var(--muted);
  box-shadow: var(--shadow-e2);
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--fg);
  cursor: default;
  max-width: min(320px, calc(100vw - var(--space-24)));
}
.toast-icon { flex-shrink: 0; color: var(--muted); }
.toast-item.success { border-left-color: var(--success-text); }
.toast-item.success .toast-icon { color: var(--success-text); }
.toast-item.error { border-left-color: var(--danger); }
.toast-item.error .toast-icon { color: var(--danger); }
.toast-item.warning { border-left-color: var(--warning-text); }
.toast-item.warning .toast-icon { color: var(--warning-text); }
.toast-item.info { border-left-color: var(--muted); }

.toast-msg { flex: 1; line-height: 1.4; }
.toast-action {
  border: none;
  background: none;
  color: var(--accent-strong);
  font-size: var(--text-xs);
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
  /* [m-20] 原来 opacity:0.4 把叉号对比度压到约 2.3:1（低于非文本 3:1 门槛）。
     改成实色 --muted（各主题均 ≥4.5:1），用颜色而非透明度表达层级。 */
  color: var(--muted);
  cursor: pointer;
  padding: var(--space-2);
  display: flex;
  align-items: center;
  transition: color var(--dur-fast);
}
.toast-close:hover { color: var(--fg); }

/* 过渡 */
.toast-enter-active { transition: opacity var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-out); }
.toast-leave-active { transition: opacity var(--dur-fast) var(--ease-in), transform var(--dur-fast) var(--ease-in); }
.toast-enter-from { opacity: 0; transform: translateX(12px); }
.toast-leave-to { opacity: 0; transform: translateX(12px); }
.toast-move { transition: transform var(--dur-base) var(--ease-out); }
</style>
