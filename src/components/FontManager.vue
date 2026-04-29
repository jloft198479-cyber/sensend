<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface UserFont {
  name: string
  path: string
}

const fonts = ref<UserFont[]>([])
const loading = ref(false)
const deletingFont = ref<string | null>(null)

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
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#d4d4d8"
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
            <span class="fm-font-name" :style="{ fontFamily: `'${font.name}', sans-serif` }">{{ font.name }}</span>
            <span class="fm-font-ext">{{ font.path.split('.').pop()?.toUpperCase() }}</span>
          </div>
          <button class="fm-delete-btn"
            :disabled="deletingFont === font.name"
            @click="deleteFont(font.name)"
            title="删除字体" aria-label="删除字体">
            <svg v-if="deletingFont !== font.name" width="12" height="12" viewBox="0 0 24 24" fill="none"
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
  background: rgba(0, 0, 0, 0.3);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
  backdrop-filter: blur(2px);
}
.font-manager {
  width: 340px;
  max-height: 420px;
  background: var(--bg);
  border-radius: 14px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), 0 2px 8px rgba(0, 0, 0, 0.06);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: fm-in 0.2s ease-out;
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
  font-size: 15px;
  font-weight: 650;
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
  transition: all 0.15s;
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
  font-size: 11.5px;
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
  border: 1px solid var(--gray-border);
  border-radius: 7px;
  background: var(--bg);
  font-size: 12px;
  font-weight: 500;
  color: var(--fg-secondary);
  cursor: pointer;
  transition: all 0.15s;
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
  border: 1px solid var(--gray-border);
  border-radius: 7px;
  background: var(--bg);
  color: var(--muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
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
  scrollbar-color: rgba(0,0,0,0.08) transparent;
}
.fm-list::-webkit-scrollbar { width: 4px; }
.fm-list::-webkit-scrollbar-track { background: transparent; }
.fm-list::-webkit-scrollbar-thumb { background: rgba(0,0,0,0.08); border-radius: 2px; }

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
  font-size: 13px;
  font-weight: 500;
}
.fm-empty-hint {
  font-size: 11px !important;
  font-weight: 400 !important;
  color: var(--gray-placeholder) !important;
}

.fm-font-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  margin-bottom: 6px;
  transition: all 0.15s;
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
  font-size: 9px;
  font-weight: 600;
  color: var(--muted);
  background: var(--gray-hover);
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}

.fm-delete-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
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


</style>