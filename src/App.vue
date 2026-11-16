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
import { computed, onMounted, ref } from 'vue'

// ── Composables ──
import { useSensendEditor } from './composables/useEditor'
import { useEditorFormat } from './composables/useEditorFormat'
import { useEditorFont } from './composables/useEditorFont'
import { usePlatform } from './composables/usePlatform'
import { useHotkey } from './composables/useHotkey'
import { useToast } from './composables/useToast'
import { useUpdater } from './composables/useUpdater'

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
  markSent,
  flushSave,
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
  toggleTaskList,
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

// ── 自动更新 ──
const { silentCheckForUpdate, manualCheck } = useUpdater()

// 主窗口 visible:false 启动（消白屏），前端挂载完成后再显示
onMounted(async () => {
  try { await getCurrentWindow().show() } catch (e) { console.error('显示窗口失败:', e) }
  // 启动静默检查更新：延迟 8s 避开启动竞速，内部 24h 限频 + 失败静默；
  // flushSave 透传给安装链（安装器直接退出应用，笔记必须先落盘）
  setTimeout(() => { silentCheckForUpdate(flushSave) }, 8000)
})

/** 设置菜单「检查更新」：发现新版即下载安装，安装前 flushSave 双保险落盘 */
function handleCheckUpdate() {
  manualCheck(flushSave)
}

// ── mention ↔ 底栏双向同步 ──

// 编辑区 mention 变化 → 同步底栏 activeInstanceId
setOnMentionChange((mentionId) => {
  if (mentionId) {
    activeInstanceId.value = mentionId
    // 存后端 config.json，fire-and-forget 不阻塞 UI
    invoke('set_default_target', { targetId: mentionId }).catch(e => {
      console.error('保存默认目标失败:', e)
    })
  }
})

// resolvedTarget: 单一解析入口，mention 优先于底栏默认目标
const resolvedTarget = computed(() => {
  const mentionId = getMentionId()
  if (mentionId) return instances.value.find(i => i.id === mentionId) || null
  return activeInstance.value || null
})

async function publishNote() {
  const ok = await platformPublishNote(editor.value, resolvedTarget.value?.id ?? null)
  if (ok) markSent(editor.value)
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
const { error: toastError, success: toastSuccess } = useToast()

async function openDataDir() {
  try {
    await invoke('open_data_dir')
  } catch (e) {
    console.error('打开数据目录失败:', e)
  }
}

// ── 一键清空笔记（可撤销：Ctrl+Z 或 toast 撤销按钮）──
function handleClear() {
  if (!editor.value) return
  editor.value.commands.clearContent()
  toastSuccess('已清空笔记', {
    label: '撤销',
    onClick: () => editor.value?.commands.undo(),
  })
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
      <BubbleMenu v-if="editor" :editor="editor">
        <div class="bubble-menu">
          <button class="bm-btn" :class="{ active: isActive('bold') }" @click="toggleBold" title="粗体" aria-label="粗体">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 4h8a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/><path d="M6 12h9a4 4 0 0 1 4 4 4 4 0 0 1-4 4H6z"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('italic') }" @click="toggleItalic" title="斜体" aria-label="斜体">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="19" y1="4" x2="10" y2="4"/><line x1="14" y1="20" x2="5" y2="20"/><line x1="15" y1="4" x2="9" y2="20"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('strike') }" @click="toggleStrike" title="删除线" aria-label="删除线">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.3 4.9c-2.3-.6-4.4-1-6.2-.9-2.7 0-5.3.7-5.3 3.6 0 1.4 1 2.3 1.7 2.9h8.2"/><path d="M2.2 10.4c1.6 1.7 4 2.7 7.2 2.7 3.5 0 5.8-.5 7.5-1.2"/><line x1="2" y1="12" x2="22" y2="12"/></svg>
          </button>
          <div class="bm-divider"></div>
          <button class="bm-btn" :class="{ active: isActive('heading', { level: 1 }) }" @click="toggleH1" title="大标题" aria-label="大标题">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12h8"/><path d="M4 18V6"/><path d="M12 18V6"/><path d="M21 18h-4c0-4 4-3 4-6 0-1.5-2-2.5-4-1"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('heading', { level: 2 }) }" @click="toggleH2" title="小标题" aria-label="小标题">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12h8"/><path d="M4 18V6"/><path d="M12 18V6"/><path d="M17.5 10.5c1.7-1 3.5 0 3.5 1.5a2 2 0 0 1-2 2 2 2 0 0 1-2-2c0-1.5 1.8-2.5 3.5-1.5"/></svg>
          </button>
          <div class="bm-divider"></div>
          <button class="bm-btn" :class="{ active: isActive('bulletList') }" @click="toggleBulletList" title="无序列表" aria-label="无序列表">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><circle cx="3.5" cy="6" r="1.5" fill="currentColor"/><circle cx="3.5" cy="12" r="1.5" fill="currentColor"/><circle cx="3.5" cy="18" r="1.5" fill="currentColor"/></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('orderedList') }" @click="toggleOrderedList" title="有序列表" aria-label="有序列表">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="10" y1="6" x2="21" y2="6"/><line x1="10" y1="12" x2="21" y2="12"/><line x1="10" y1="18" x2="21" y2="18"/><text x="2" y="8" font-size="8" font-weight="600" fill="currentColor" stroke="none">1</text><text x="2" y="14" font-size="8" font-weight="600" fill="currentColor" stroke="none">2</text><text x="2" y="20" font-size="8" font-weight="600" fill="currentColor" stroke="none">3</text></svg>
          </button>
          <button class="bm-btn" :class="{ active: isActive('taskList') }" @click="toggleTaskList" title="待办" aria-label="待办">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="3"/><path d="M9 12l2 2 4-4"/></svg>
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
      @clear="handleClear"
      @check-update="handleCheckUpdate"
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
  color: var(--accent-strong);
  background: var(--accent-light);
  border: 1px solid color-mix(in srgb, var(--accent) 15%, transparent);
  border-radius: var(--radius-sm);
  padding: 1px 6px;
  font-size: var(--text-xs);
  font-weight: 500;
  cursor: default;
  user-select: none;
  transition: all var(--dur-fast) ease;
}
.platform-mention:hover {
  background: color-mix(in srgb, var(--accent) 10%, var(--accent-light));
  border-color: color-mix(in srgb, var(--accent) 25%, transparent);
}

/* ═══ 待办列表（taskList/taskItem）═══ */
/* 注：li 用 [data-checked] 而非 [data-type="taskItem"] 匹配 ——
   extension-list 3.28 的 NodeView 只给 li 设 data-checked，不设 data-type */
ul[data-type="taskList"] {
  list-style: none;
  margin: 0.2em 0;
  padding: 0;
}
ul[data-type="taskList"] li[data-checked] {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 0.2em 0;
}
ul[data-type="taskList"] li[data-checked] > label {
  flex: none;
  margin-top: 2px;
  cursor: pointer;
  user-select: none;
}
ul[data-type="taskList"] li[data-checked] > label input {
  width: 16px;
  height: 16px;
  margin: 0;
  accent-color: var(--accent);
  cursor: pointer;
}
ul[data-type="taskList"] li[data-checked] > div {
  flex: 1;
  min-width: 0;
}
ul[data-type="taskList"] li[data-checked] > div p {
  margin: 0;
}
/* 已完成项：文字置灰 */
ul[data-type="taskList"] li[data-checked="true"] > div {
  color: var(--muted);
  text-decoration: line-through;
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
/* 窄窗优先（方案 4.11）：280–359 与 360–419 两档收紧内边距，420 以上保持舒展版 */
@media (max-width: 419px) {
  .editor-wrapper { padding: 24px 20px; }
}
@media (max-width: 359px) {
  .editor-wrapper { padding: 20px 16px; }
}
/* 仅最小高度 200px 时压缩上下留白（默认窗高 210 不受影响） */
@media (max-height: 200px) {
  .editor-wrapper { padding-top: 12px; padding-bottom: 12px; }
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

/* ═══ BubbleMenu ═══ */
.bubble-menu {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: center;
  gap: 1px;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-e2);
  padding: 3px 4px;
  /* 窄窗不横向溢出：放不下时折成第二行（280px 下仍保留全部动作） */
  max-width: calc(100vw - 16px);
}
.bm-btn {
  width: 28px; height: 28px;
  border: none; background: transparent;
  color: var(--fg-secondary); cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex; align-items: center; justify-content: center;
  transition: all var(--dur-fast) ease; padding: 0;
}
.bm-btn:hover { background: var(--bg-hover); color: var(--fg); }
.bm-btn.active { background: var(--accent-light); color: var(--accent-strong); }
.bm-divider {
  width: 1px; height: 16px;
  background: var(--border-color);
  margin: 0 3px; flex-shrink: 0;
}

</style>