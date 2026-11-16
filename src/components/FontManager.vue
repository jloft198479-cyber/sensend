<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface UserFont {
  name: string
  path: string
}

const fonts = ref<UserFont[]>([])
const loading = ref(false)
const deletingFont = ref<string | null>(null)
// 删除二次确认：第一次点击替换为"确认删除"，3 秒未操作自动回退（方案 M-6）
const confirmingFont = ref<string | null>(null)
let confirmTimer: ReturnType<typeof setTimeout> | null = null

function requestDelete(name: string) {
  if (confirmTimer) { clearTimeout(confirmTimer); confirmTimer = null }
  if (confirmingFont.value === name) {
    confirmingFont.value = null
    void deleteFont(name)
    return
  }
  confirmingFont.value = name
  confirmTimer = setTimeout(() => { confirmingFont.value = null }, 3000)
}

onBeforeUnmount(() => { if (confirmTimer) clearTimeout(confirmTimer) })

const emit = defineEmits<{
  close: []
  refreshed: [fonts: UserFont[]]
}>()

async function loadFonts() {
  loading.value = true
  try {
    fonts.value = await invoke<UserFont[]>('scan_user_fonts')
  } catch (e) {
    console.error('加载字体列表失败:', e)
  } finally {
    loading.value = false
  }
}

async function deleteFont(name: string) {
  deletingFont.value = name
  try {
    await invoke('delete_user_font', { fontName: name })
    await loadFonts()
    emit('refreshed', fonts.value)
  } catch (e) {
    console.error('删除字体失败:', e)
  } finally {
    deletingFont.value = null
    confirmingFont.value = null
  }
}

async function openFontsDir() {
  try {
    await invoke('open_fonts_dir')
  } catch (e) {
    console.error('打开字体目录失败:', e)
  }
}

onMounted(loadFonts)
</script>

<template>
  <div class="font-manager-overlay" @click.self="$emit('close')">
    <div class="font-manager">
      <!-- 标题栏 -->
      <div class="fm-header">
        <h3>字体管理</h3>
        <button class="fm-close" @click="$emit('close')" aria-label="关闭">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>

      <!-- 说明 -->
      <div class="fm-hint">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--muted)"
          stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
        </svg>
        <span>将字体文件（.ttf / .otf / .woff2）放入字体文件夹，重启后自动加载</span>
      </div>

      <!-- 操作栏 -->
      <div class="fm-actions">
        <button class="fm-folder-btn" @click="openFontsDir" aria-label="打开字体文件夹">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
          </svg>
          <span>打开字体文件夹</span>
        </button>
        <button class="fm-refresh-btn" @click="loadFonts" :disabled="loading" aria-label="刷新字体列表">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2" stroke-linecap="round" stroke-linejoin="round"
            :class="{ 'spin-icon': loading }">
            <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
          </svg>
        </button>
      </div>

      <!-- 字体列表 -->
      <div class="fm-list">
        <div v-if="fonts.length === 0" class="fm-empty">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--gray-scrollbar)"
            stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 7 4 4 20 4 20 7"/>
            <line x1="9" y1="20" x2="15" y2="20"/>
            <line x1="12" y1="4" x2="12" y2="20"/>
          </svg>
          <p>还没有自定义字体</p>
          <p class="fm-empty-hint">将 .ttf 或 .otf 文件放入字体文件夹即可</p>
        </div>
        <div v-for="font in fonts" :key="font.name" class="fm-font-item">
          <div class="fm-font-info">
            <!-- 预览不套自定义字体：多 CJK 字体并发解析（单个 ttf 数 MB～几十 MB）会卡开列表；
               实际字形效果在编辑器内选择该字体即可看到 -->
          <span class="fm-font-name">{{ font.name }}</span>
            <span class="fm-font-ext">{{ font.path.split('.').pop()?.toUpperCase() }}</span>
          </div>
          <button class="fm-delete-btn" :class="{ confirming: confirmingFont === font.name }"
            :disabled="deletingFont === font.name"
            @click="requestDelete(font.name)"
            :title="confirmingFont === font.name ? '再次点击确认删除' : '删除字体'"
            :aria-label="confirmingFont === font.name ? '确认删除字体' : '删除字体'">
            <span v-if="confirmingFont === font.name && deletingFont !== font.name" class="fm-confirm-text">确认删除</span>
            <svg v-else-if="deletingFont !== font.name" width="12" height="12" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 6 5 6 21 6"/>
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
            </svg>
            <svg v-else class="spin-icon" width="12" height="12" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <path d="M21 12a9 9 0 11-6.219-8.56"/>
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.font-manager-overlay {
  position: fixed;
  inset: 0;
  background: var(--scrim-bg);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-scrim);
  backdrop-filter: blur(2px);
}
.font-manager {
  position: relative;
  z-index: var(--z-modal);
  width: 360px;
  max-height: 420px;
  background: var(--bg);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-e3);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: fm-in var(--dur-slow) var(--ease-out);
}
@keyframes fm-in {
  from { opacity: 0; transform: scale(0.96) translateY(8px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

/* 标题栏 */
.fm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px 12px;
}
.fm-header h3 {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--fg);
  margin: 0;
}
.fm-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast);
  padding: 0;
}
.fm-close:hover {
  background: var(--bg-hover);
  color: var(--fg);
}

/* 说明 */
.fm-hint {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  padding: 0 18px 12px;
  font-size: var(--text-xs);
  color: var(--muted);
  line-height: 1.5;
}
.fm-hint svg {
  flex-shrink: 0;
  margin-top: 1px;
}

/* 操作栏 */
.fm-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 18px 12px;
}
.fm-folder-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 12px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-md);
  background: var(--bg);
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--fg-secondary);
  cursor: pointer;
  transition: all var(--dur-fast);
  font-family: var(--font-sans);
}
.fm-folder-btn:hover {
  background: var(--gray-hover);
  border-color: var(--gray-scrollbar);
  color: var(--fg);
}
.fm-refresh-btn {
  width: 28px;
  height: 28px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-md);
  background: var(--bg);
  color: var(--muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast);
  padding: 0;
}
.fm-refresh-btn:hover:not(:disabled) {
  background: var(--gray-hover);
  color: var(--fg);
}
.fm-refresh-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* 字体列表 */
.fm-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 18px 16px;
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) transparent;
}
.fm-list::-webkit-scrollbar { width: 4px; }
.fm-list::-webkit-scrollbar-track { background: transparent; }
.fm-list::-webkit-scrollbar-thumb { background: var(--scrollbar-thumb); border-radius: 2px; }

.fm-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 28px 20px;
  color: var(--muted);
  text-align: center;
}
.fm-empty p {
  margin: 8px 0 0;
  font-size: var(--text-sm);
  font-weight: 500;
}
.fm-empty-hint {
  font-size: var(--text-xs) !important;
  font-weight: 400 !important;
  color: var(--placeholder) !important;
}

.fm-font-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  margin-bottom: 6px;
  transition: all var(--dur-fast);
}
.fm-font-item:hover {
  border-color: var(--gray-border);
  background: var(--gray-hover);
}
.fm-font-item:last-child { margin-bottom: 0; }

.fm-font-info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.fm-font-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fm-font-ext {
  height: 18px;
  line-height: 18px;
  padding: 0 6px;
  font-size: var(--text-2xs);
  font-weight: 600;
  color: var(--accent-strong);
  background: var(--accent-light);
  border-radius: var(--radius-xs);
  flex-shrink: 0;
}

.fm-delete-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast);
  padding: 0;
  flex-shrink: 0;
}
.fm-delete-btn:hover:not(:disabled) {
  background: var(--danger-bg);
  color: var(--danger);
}
.fm-delete-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
/* 二次确认态：按钮原地替换为文字，3 秒未点自动回退 */
.fm-delete-btn.confirming {
  width: auto;
  height: 24px;
  padding: 0 8px;
  color: var(--danger);
  background: var(--danger-bg);
  border: 1px solid var(--danger);
}
.fm-confirm-text {
  font-size: var(--text-2xs);
  font-weight: 600;
  white-space: nowrap;
}


</style>