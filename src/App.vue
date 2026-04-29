<script setup lang="ts">
// ── 子组件 ──
import TitleBar from './components/TitleBar.vue'
import FooterBar from './components/FooterBar.vue'
import HotkeyModal from './components/HotkeyModal.vue'
import FontManager from './components/FontManager.vue'
import ToastLayer from './components/ToastLayer.vue'
import { EditorContent } from '@tiptap/vue-3'
import { BubbleMenu } from '@tiptap/vue-3/menus'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { computed, ref } from 'vue'

// ── Composables ──
import { useSensendEditor } from './composables/useEditor'
import { useEditorFormat } from './composables/useEditorFormat'
import { useEditorFont } from './composables/useEditorFont'
import { usePlatform } from './composables/usePlatform'
import { useHotkey } from './composables/useHotkey'
import { useToast } from './composables/useToast'

// ── 平台逻辑 ──
const {
  instances,
  activeInstanceId,
  activeInstance,
  platformTypes,
  isSending,
  selectTarget,
  openConfigWindow,
  publishNote: platformPublishNote,
} = usePlatform()

// ── 编辑器逻辑 ──
const {
  editor,
  saveStatus,
  wordCount,
  charCount,
  getMentionId,
  setMention,
  setOnMentionChange,
} = useSensendEditor(instances, platformTypes)

// ── 格式操作 ──
const {
  toggleBold,
  toggleItalic,
  toggleStrike,
  toggleH1,
  toggleH2,
  toggleBulletList,
  toggleOrderedList,
  toggleBlockquote,
  toggleCode,
  isActive,
} = useEditorFormat(editor)

// ── 字体管理 ──
const {
  fontOptions,
  currentFont,
  showFontMenu,
  showFontManager,
  selectFont,
  addFont,
  toggleFontMenu,
  onFontManagerRefreshed,
} = useEditorFont()

// ── 窗口操作 ──
const isPinned = ref(true)

async function togglePin() {
  isPinned.value = !isPinned.value
  try { await getCurrentWindow().setAlwaysOnTop(isPinned.value) } catch {}
}

async function hideWindow() { await invoke('hide_window') }

// ── mention ↔ 底栏双向同步 ──

// 编辑区 mention 变化 → 同步底栏 activeInstanceId
setOnMentionChange((mentionId) => {
  if (mentionId) {
    activeInstanceId.value = mentionId
    localStorage.setItem('sensend-default-target', mentionId)
  }
})

// resolvedTarget: 单一解析入口，mention 优先于底栏默认目标
const resolvedTarget = computed(() => {
  const mentionId = getMentionId()
  if (mentionId) return instances.value.find(i => i.id === mentionId) || null
  return activeInstance.value || null
})

function publishNote() {
  platformPublishNote(editor.value, resolvedTarget.value?.id ?? null)
}

// 底栏选择 → 同步编辑区 mention
function handleFooterSelect(id: string) {
  selectTarget(id)
  setMention(id)
}

const {
  showHotkeyModal,
  hotkeyForm,
  hotkeyEditState,
  hotkeySaveStatus,
  openHotkeyModal,
  startRecordingHotkey,
  onKeyDownForHotkey,
  saveHotkeys,
} = useHotkey(publishNote, () => editor.value, isSending)

// ── 快捷键保存失败反馈 ──
const { error: toastError } = useToast()

async function openDataDir() {
  try {
    await invoke('open_data_dir')
  } catch (e) {
    console.error('打开数据目录失败:', e)
  }
}

async function onSaveHotkeys() {
  try {
    await saveHotkeys()
  } catch (e: any) {
    toastError('快捷键保存失败: ' + (e?.message || String(e)))
  }
}
</script>

<template>
  <div class="sensend">
    <!-- ═══ 顶栏 ═══ -->
    <TitleBar
      :is-pinned="isPinned"
      :is-sending="isSending"
      :show-font-menu="showFontMenu"
      :font-options="fontOptions"
      :current-font="currentFont"
      :save-status="saveStatus"
      :active-instance-id="activeInstanceId"
      :instances="instances"
      @toggle-pin="togglePin"
      @publish="publishNote"
      @hide="hideWindow"
      @toggle-font-menu="toggleFontMenu"
      @select-font="selectFont"
      @add-font="addFont"
      @open-config="openConfigWindow"
    />

    <!-- ═══ Toast 浮层 ═══ -->
    <ToastLayer />

    <!-- ═══ 编辑区 ═══ -->
    <div class="editor-wrapper">
      <BubbleMenu v-if="editor" :editor="editor" :tippy-options="{ duration: 150 }">
        <div class="bubble-menu">
          <button class="bm-btn" :class="{ active: isActive('bold') }" @click="toggleBold" title="粗体" aria-label="粗体">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/><path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('italic') }" @click="toggleItalic" title="斜体" aria-label="斜体">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="4" x2="10" y2="4"/><line x1="14" y1="20" x2="5" y2="20"/><line x1="15" y1="4" x2="9" y2="20"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('strike') }" @click="toggleStrike" title="删除线" aria-label="删除线">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.3 4.9c-2.3-.6-4.4-1-6.2-.9-2.7 0-5.3.7-5.3 3.6 0 1.4 1 2.3 1.7 2.9h8.2"/><path d="M2.2 10.4c1.6 1.7 4 2.7 7.2 2.7 3.5 0 5.8-.5 7.5-1.2"/><line x1="2" y1="12" x2="22" y2="12"/></svg>
          </button>
          <div class="bm-divider"></div>
          <button class="bm-btn" :class="{ active: isActive('heading', { level: 1 }) }" @click="toggleH1" title="大标题" aria-label="大标题">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12h8"/><path d="M4 18V6"/><path d="M12 18V6"/><path d="M21 18h-4c0-4 4-3 4-6 0-1.5-2-2.5-4-1"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('heading', { level: 2 }) }" @click="toggleH2" title="小标题" aria-label="小标题">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12h8"/><path d="M4 18V6"/><path d="M12 18V6"/><path d="M17.5 10.5c1.7-1 3.5 0 3.5 1.5a2 2 0 0 1-2 2 2 2 0 0 1-2-2c0-1.5 1.8-2.5 3.5-1.5"/></svg>
          </button>
          <div class="bm-divider"></div>
          <button class="bm-btn" :class="{ active: isActive('bulletList') }" @click="toggleBulletList" title="无序列表" aria-label="无序列表">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><circle cx="3.5" cy="6" r="1.5" fill="currentColor"/><circle cx="3.5" cy="12" r="1.5" fill="currentColor"/><circle cx="3.5" cy="18" r="1.5" fill="currentColor"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('orderedList') }" @click="toggleOrderedList" title="有序列表" aria-label="有序列表">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="10" y1="6" x2="21" y2="6"/><line x1="10" y1="12" x2="21" y2="12"/><line x1="10" y1="18" x2="21" y2="18"/><text x="2" y="8" font-size="8" font-weight="600" fill="currentColor" stroke="none">1</text><text x="2" y="14" font-size="8" font-weight="600" fill="currentColor" stroke="none">2</text><text x="2" y="20" font-size="8" font-weight="600" fill="currentColor" stroke="none">3</text></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('blockquote') }" @click="toggleBlockquote" title="引用" aria-label="引用">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V21z"/><path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3c0 1 0 1 1 1z"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('code') }" @click="toggleCode" title="行内代码" aria-label="行内代码">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
          </button>
        </div>
      </BubbleMenu>

      <EditorContent :editor="editor" v-if="editor" />
    </div>

    <!-- ═══ 底栏（当前发送目标 + 字数统计）═══ -->
    <FooterBar
      :instances="instances"
      :platform-types="platformTypes"
      :current-target="resolvedTarget"
      :word-count="wordCount"
      :char-count="charCount"
      @open-config="openConfigWindow"
      @open-hotkey="openHotkeyModal"
      @open-data-dir="openDataDir"
      @select-target="handleFooterSelect"
    />

    <!-- ═══ 快捷键设置弹窗 ═══ -->
    <HotkeyModal
      :visible="showHotkeyModal"
      :hotkey-form="hotkeyForm"
      :hotkey-edit-state="hotkeyEditState"
      :hotkey-save-status="hotkeySaveStatus"
      @close="showHotkeyModal = false"
      @start-recording="startRecordingHotkey"
      @save="onSaveHotkeys"
      @keydown="onKeyDownForHotkey"
    />

    <!-- ═══ 字体管理弹窗 ═══ -->
    <FontManager
      v-if="showFontManager"
      @close="showFontManager = false"
      @refreshed="onFontManagerRefreshed"
    />
  </div>
</template>

<!-- 编辑器字体：默认使用系统字体栈，用户可通过字体选择器切换系统已安装字体 -->

<style>
/* ═══ @平台 Mention ═══ */
.platform-mention {
  color: var(--accent);
  background: linear-gradient(135deg, var(--accent-light) 0%, rgba(44, 175, 104, 0.03) 100%);
  border: 1px solid rgba(44, 175, 104, 0.15);
  border-radius: 5px;
  padding: 1px 6px;
  font-size: 0.92em;
  font-weight: 500;
  cursor: default;
  user-select: none;
  transition: all 0.15s ease;
}
.platform-mention:hover {
  background: linear-gradient(135deg, rgba(44, 175, 104, 0.12) 0%, rgba(44, 175, 104, 0.06) 100%);
  border-color: rgba(44, 175, 104, 0.25);
}
</style>

<style scoped>
/* ═══ 容器 ═══ */
.sensend {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* ═══ 编辑器容器 ═══ */
.editor-wrapper {
  flex: 1;
  padding: 32px 28px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: transparent transparent;
  min-height: 0;
  transition: scrollbar-color 0.3s;
}
.editor-wrapper:hover {
  scrollbar-color: var(--scrollbar-thumb) transparent;
}
.editor-wrapper::-webkit-scrollbar { width: 4px; }
.editor-wrapper::-webkit-scrollbar-track { background: transparent; }
.editor-wrapper::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 2px;
  transition: background 0.3s;
}
.editor-wrapper:hover::-webkit-scrollbar-thumb { background: var(--scrollbar-thumb); }
.editor-wrapper:hover::-webkit-scrollbar-thumb:hover { background: var(--scrollbar-thumb-hover); }

/* ═══ TipTap 编辑器样式已抽取至 styles/editor.css ═══ */

/* 下拉动画 */
.dropdown-enter-active { transition: all 0.15s ease-out; }
.dropdown-leave-active { transition: all 0.1s ease-in; }
.dropdown-enter-from { opacity: 0; transform: translateY(4px); }
.dropdown-leave-to { opacity: 0; transform: translateY(4px); }

/* ═══ BubbleMenu ═══ */
.bubble-menu {
  display: flex;
  align-items: center;
  gap: 1px;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  box-shadow: 0 4px 20px rgba(0,0,0,0.08), 0 1px 4px rgba(0,0,0,0.04);
  padding: 3px 4px;
}
.bm-btn {
  width: 28px; height: 28px;
  border: none; background: transparent;
  color: var(--fg-secondary); cursor: pointer;
  border-radius: 6px;
  display: flex; align-items: center; justify-content: center;
  transition: all 0.12s ease; padding: 0;
}
.bm-btn:hover { background: var(--bg-hover); color: var(--fg); }
.bm-btn.active { background: var(--accent-light); color: var(--accent); }
.bm-divider {
  width: 1px; height: 16px;
  background: var(--gray-border);
  margin: 0 3px; flex-shrink: 0;
}

/* tippy mention 主题：去掉默认样式，让 MentionList 自己控制外观 */
.tippy-box[data-theme~='mention'] {
  background: transparent;
  box-shadow: none;
}
.tippy-box[data-theme~='mention'] .tippy-content {
  padding: 0;
}

</style>