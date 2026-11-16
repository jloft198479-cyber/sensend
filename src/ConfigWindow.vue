<script setup lang="ts">
import { ref, onMounted, watch, onBeforeUnmount } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useConfig } from './composables/useConfig'
import ToastLayer from './components/ToastLayer.vue'

const {
  instances,
  platformTypes,
  showForm,
  editingInstanceId,
  configForm,
  isTesting,
  testResult,
  testErrorMsg,
  showDeleteConfirm,
  showToken,
  savePulse,
  currentPlatformFields,
  currentModes,
  canSave,
  loadData,
  openAddModal,
  openEditModal,
  browseFolder,
  onPlatformTypeChange,
  testConnection,
  saveInstance,
  deleteInstance,
} = useConfig()

// 无边框窗口：顶栏作为拖拽区（交互区 .title-actions 排除），关闭走自绘按钮
const titlebarRef = ref<HTMLElement | null>(null)
const closeWindow = () => { getCurrentWindow().close() }

// 删除二次确认：3 秒未操作自动回退（方案 M-6，与清空笔记同一交互）
let deleteTimer: ReturnType<typeof setTimeout> | null = null
watch(showDeleteConfirm, (id) => {
  if (deleteTimer) { clearTimeout(deleteTimer); deleteTimer = null }
  if (id) deleteTimer = setTimeout(() => { showDeleteConfirm.value = null }, 3000)
})
onBeforeUnmount(() => { if (deleteTimer) clearTimeout(deleteTimer) })

onMounted(() => {
  loadData()
  const bar = titlebarRef.value
  if (!bar) return
  const applyDrag = (el: HTMLElement) => {
    if (el.closest('.title-actions')) return
    el.setAttribute('data-tauri-drag-region', '')
    Array.from(el.children).forEach((child) => {
      if (child instanceof HTMLElement) applyDrag(child)
    })
  }
  applyDrag(bar)
})
</script>

<template>
  <div class="config-app">
    <!-- ═══ 顶栏 ═══ -->
    <header ref="titlebarRef" class="config-titlebar">
      <div class="title-actions">
        <button v-if="showForm" class="title-btn back-title-btn" @click="showForm = false" title="返回列表" aria-label="返回列表">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          <span>返回</span>
        </button>
        <button v-else class="title-btn add-new-btn" @click="openAddModal" aria-label="新增页面">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          <span>新增页面</span>
        </button>
        <button class="close-win-btn" @click="closeWindow" title="关闭" aria-label="关闭">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    </header>

    <!-- ═══ 视图切换：编辑表单 / 页面列表（out-in，旧视图退场后新视图再入场，同屏仅一个）═══ -->
    <Transition name="view" mode="out-in">
    <div v-if="showForm" key="form" class="edit-section">

      <div class="edit-form">
        <div class="form-group">
          <div class="form-group-title">基础信息</div>
        <div class="form-row">
          <label class="form-label">名称</label>
          <input v-model="configForm.name" type="text" class="form-input"
            placeholder="如：工作 Notion、个人 FlowUs..." spellcheck="false" />
        </div>
        <div class="form-row">
          <label class="form-label">平台类型</label>
          <div class="platform-select">
            <select v-model="configForm.platform_type" @change="onPlatformTypeChange"
              :disabled="!!editingInstanceId">
              <option v-for="pt in platformTypes" :key="pt.key" :value="pt.key">{{ pt.name }}</option>
            </select>
          </div>
        </div>
        </div>
        <div class="form-group">
          <div class="form-group-title">连接凭证</div>
        <template v-for="field in currentPlatformFields" :key="field.key">
          <div v-if="!field.hidden" class="form-row">
            <label class="form-label">
              {{ field.label }}
              <button v-if="field.browse" class="browse-btn" @click="browseFolder" aria-label="浏览文件夹">浏览…</button>
            </label>
            <div class="input-with-toggle">
              <input v-model="(configForm as any)[field.key]"
                :type="field.secret && !showToken ? 'password' : 'text'" class="form-input"
                :class="{ 'input-mono': ['token', 'token2', 'target_id'].includes(field.key) }"
                placeholder="粘贴..." spellcheck="false" autocomplete="off" />
              <button v-if="field.secret" class="toggle-visibility" @click="showToken = !showToken"
                :title="showToken ? '隐藏' : '显示'" :aria-label="showToken ? '隐藏密钥' : '显示密钥'" tabindex="-1">
                <svg v-if="!showToken" width="14" height="14" viewBox="0 0 24 24" fill="none"
                  stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
                </svg>
                <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none"
                  stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
                  <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
                  <line x1="1" y1="1" x2="23" y2="23"/>
                </svg>
              </button>
            </div>
            <div v-if="field.hint" class="field-hint">{{ field.hint }}</div>
          </div>
        </template>
        </div>

        <!-- 写入方式：平台是否支持多档由后端 supported_modes 下发（单档平台已在 useConfig 归一，无需选择） -->
        <div v-if="currentModes.length > 1" class="form-group">
          <div class="form-group-title">写入方式</div>
          <div class="form-row">
            <div class="platform-select">
              <select v-model="configForm.publish_mode">
                <option value="page">创建子页面</option>
                <option value="block">追加到页面</option>
              </select>
            </div>
          </div>
        </div>

        <div class="form-actions">
          <button class="action-btn test-btn" :disabled="!canSave || isTesting"
            @click="testConnection" aria-label="测试连接">
            <svg v-if="isTesting" class="spin-icon" width="14" height="14" viewBox="0 0 24 24" fill="none"
              stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12a9 9 0 11-6.219-8.56"/>
            </svg>
            <span v-else>测试连接</span>
          </button>
          <button class="action-btn save-btn" :class="{ 'btn-bounce': savePulse }" :disabled="!canSave" @click="saveInstance" aria-label="保存">
            {{ editingInstanceId ? '更新' : '保存' }}
          </button>
        </div>
        <Transition name="fade">
          <div v-if="testResult === 'ok'" class="test-msg success">✓ 连接成功，可以保存了</div>
          <div v-else-if="testResult === 'fail'" class="test-msg fail">
            ✕ {{ testErrorMsg || '连接失败，请检查配置' }}
          </div>
        </Transition>
      </div>
    </div>

    <!-- ═══ 已配置页面列表 ═══ -->
    <div v-else key="list" class="list-section">
      <div v-if="instances.length === 0" class="empty-state">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--gray-scrollbar)"
          stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="3" width="20" height="18" rx="2"/>
          <path d="M12 8v8M8 12h8"/>
        </svg>
        <p>还没有配置任何发送页面</p>
        <p class="empty-hint">点击上方「新增页面」开始配置</p>
      </div>
      <div v-else class="instance-list">
        <div v-for="inst in instances" :key="inst.id" class="instance-card">
          <div class="instance-info">
            <div class="instance-text">
              <span class="instance-name">{{ inst.name }}</span>
              <span class="instance-type">
                {{ platformTypes.find(t => t.key === inst.platform_type)?.name }}
                <span v-if="inst.publish_mode === 'block'" class="mode-tag">追加</span>
                <span v-else-if="inst.publish_mode === 'page'" class="mode-tag">子页面</span>
              </span>
            </div>
          </div>
          <div class="instance-actions">
            <button v-if="showDeleteConfirm === inst.id" class="confirm-btn delete-confirm" @click="deleteInstance(inst.id)" aria-label="确认删除">
              确认删除
            </button>
            <template v-else>
              <button class="icon-btn" @click="openEditModal(inst)" title="编辑" aria-label="编辑">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                  stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                  <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
              </button>
              <button class="icon-btn danger" @click="showDeleteConfirm = inst.id" title="删除" aria-label="删除">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                  stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6"/>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                </svg>
              </button>
            </template>
          </div>
        </div>
      </div>
    </div>
    </Transition>
    <!-- 配置窗是独立 webview（独立 JS 上下文），useToast 的队列必须由本窗口内的 ToastLayer 渲染 -->
    <ToastLayer />
  </div>
</template>

<style scoped>
.config-app {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-page);
  font-family: var(--font-sans);
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
}

/* ═══ 顶栏 ═══ */
.config-titlebar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 var(--space-14) 0 var(--space-16);
  height: var(--titlebar-h);
  background: var(--accent-light);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  cursor: default;
  user-select: none;
}
.title-actions {
  display: flex;
  align-items: center;
  gap: var(--space-8);
}
.title-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-6);
  height: var(--control-h-md);
  padding: 0 var(--space-16);
  min-width: 148px;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: 600;
  cursor: pointer;
  transition: all var(--dur-fast) ease;
  font-family: var(--font-sans);
  white-space: nowrap;
}
.title-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.add-new-btn {
  background: var(--accent-fill);
  color: var(--fg-on-accent);
}
.add-new-btn:hover:not(:disabled) {
  background: var(--accent-fill-hover);
}
.add-new-btn:active:not(:disabled) {
  transform: scale(0.98);
}
.back-title-btn {
  background: var(--gray-hover);
  color: var(--fg-secondary);
}
.back-title-btn:hover {
  background: var(--gray-border);
  color: var(--fg);
}
/* 关闭按钮（无边框窗口自绘） */
.close-win-btn {
  width: var(--btn-size);
  height: var(--btn-size);
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast) ease;
  padding: 0;
}
.close-win-btn:hover {
  background: var(--danger-bg);
  color: var(--danger);
}
.edit-section {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-20);
  background: var(--bg);
  border-bottom: 1px solid var(--border-color);
}


.edit-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-20);
}
/* 表单分区（方案 C-2 / 4.8.2）：区块间 20，区块内 14 */
.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-14);
}
.form-group-title {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
}
.form-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}
.form-label {
  font-size: var(--text-xs);
  font-weight: 600;
  color: var(--fg);
  display: flex;
  align-items: center;
  gap: var(--space-6);
}
.label-hint {
  font-size: var(--text-xs);
  font-weight: 400;
  color: var(--muted);
  font-style: italic;
}
.field-hint {
  font-size: var(--text-xs);
  color: var(--muted);
  margin-top: var(--space-2);
  line-height: 1.4;
}
.browse-btn {
  margin-left: auto;
  padding: var(--space-2) var(--space-8);
  border: 1px solid var(--control-border);
  border-radius: var(--radius-xs);
  background: var(--bg);
  font-size: var(--text-2xs);
  color: var(--fg-secondary);
  cursor: pointer;
  transition: all var(--dur-fast);
  font-family: var(--font-sans);
}
.browse-btn:hover { background: var(--gray-hover); border-color: var(--gray-scrollbar); }

.platform-select {
  position: relative;
}
/* 下拉箭头：CSS 描边绘制，随主题 --muted 变色（替代写死颜色的 data-URI，方案 C-6） */
.platform-select::after {
  content: '';
  position: absolute;
  right: var(--space-14);
  top: 50%;
  width: var(--space-8);
  height: var(--space-8);
  border-right: 2px solid var(--muted);
  border-bottom: 2px solid var(--muted);
  transform: translateY(-70%) rotate(45deg);
  pointer-events: none;
}
.platform-select select {
  width: 100%;
  height: var(--control-h-lg);
  padding: 0 var(--space-32) 0 var(--space-12);
  border: 1px solid var(--control-border);
  border-radius: var(--radius-md);
  background: var(--gray-input-bg);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  outline: none;
  appearance: none;
  font-family: var(--font-sans);
  transition: border-color var(--dur-fast);
}
.platform-select select:focus {
  border-color: var(--accent-strong);
  background-color: var(--bg);
  box-shadow: var(--shadow-focus);
}
.platform-select select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  background-color: var(--gray-hover);
}

.form-input {
  width: 100%;
  height: var(--control-h-lg);
  padding: 0 var(--space-12);
  border: 1px solid var(--control-border);
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  color: var(--fg);
  background: var(--gray-input-bg);
  outline: none;
  font-family: var(--font-sans);
  transition: border-color var(--dur-fast), box-shadow var(--dur-fast);
  box-sizing: border-box;
}
/* 凭证 / Target ID 用等宽（方案 4.8.2：名称等自然语言字段一律不用等宽） */
.form-input.input-mono { font-family: var(--font-mono); }
.form-input:focus {
  border-color: var(--accent-strong);
  background: var(--bg);
  box-shadow: var(--shadow-focus);
}
.form-input::placeholder { color: var(--placeholder); }

.input-with-toggle {
  position: relative;
  display: flex;
  align-items: center;
}
.input-with-toggle .form-input {
  padding-right: var(--control-h-lg);
}
.toggle-visibility {
  position: absolute;
  right: var(--space-4);
  width: var(--btn-size);
  height: var(--btn-size);
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
}
.toggle-visibility:hover {
  background: var(--gray-hover);
  color: var(--fg-secondary);
}

.form-actions {
  display: flex;
  gap: var(--space-10);
  margin-top: var(--space-2);
}
.action-btn {
  flex: 1;
  height: var(--control-h-md);
  padding: 0;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--text-sm);
  font-weight: 600;
  cursor: pointer;
  transition: all var(--dur-fast) ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-6);
  font-family: var(--font-sans);
}
.action-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.test-btn { background: var(--gray-hover); color: var(--fg); }
.test-btn:hover:not(:disabled) { background: var(--gray-border); }
.save-btn { background: var(--accent-fill); color: var(--fg-on-accent); }
.save-btn:hover:not(:disabled) { background: var(--accent-fill-hover); }

/* ═══ 实例列表 ═══ */
.list-section {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-16) var(--space-20);
}
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-28) var(--space-20);
  color: var(--muted);
  text-align: center;
}
.empty-state p {
  margin: 0;
  font-size: var(--text-sm);
  font-weight: 500;
}
.empty-hint {
  margin-top: var(--space-6) !important;
  font-size: var(--text-xs) !important;
  font-weight: 400 !important;
  color: var(--placeholder);
}

.instance-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.instance-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-10) var(--space-12);
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  transition: all var(--dur-fast) ease;
}
.instance-card:hover {
  border-color: var(--control-border);
  box-shadow: var(--shadow-e1);
}
.instance-info {
  display: flex;
  align-items: center;
  gap: var(--space-10);
  min-width: 0;
}
.instance-text {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}
.instance-name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.instance-type {
  font-size: var(--text-2xs);
  color: var(--muted);
  display: flex;
  align-items: center;
  gap: var(--space-6);
}
.mode-tag {
  height: 18px;
  line-height: 18px;
  padding: 0 var(--space-6);
  font-size: var(--text-2xs);
  font-weight: 600;
  color: var(--accent-strong);
  background: var(--accent-light);
  border-radius: var(--radius-xs);
}
.instance-actions {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex-shrink: 0;
}
.icon-btn {
  width: var(--btn-size);
  height: var(--btn-size);
  border: none;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--dur-fast) ease;
  padding: 0;
}
.icon-btn:hover { background: var(--gray-hover); color: var(--fg); }
.icon-btn.danger:hover { background: var(--danger-bg); color: var(--danger); }
.confirm-btn {
  height: var(--control-h-sm);
  padding: 0 var(--space-10);
  border: 1px solid var(--danger);
  background: var(--danger-bg);
  color: var(--danger);
  border-radius: var(--radius-sm);
  font-size: var(--text-2xs);
  font-weight: 600;
  cursor: pointer;
  font-family: var(--font-sans);
  transition: all var(--dur-fast);
}
.confirm-btn:hover { background: var(--danger-bg); }

/* ═══ 视图切换过渡（out-in：旧视图淡出→新视图滑入，两个方向节奏对称）═══ */
.view-enter-active { transition: opacity var(--dur-base) var(--ease-out), transform var(--dur-base) var(--ease-out); }
.view-leave-active { transition: opacity var(--dur-fast) var(--ease-in), transform var(--dur-fast) var(--ease-in); }
.view-enter-from { opacity: 0; transform: translateY(-6px); }
.view-leave-to { opacity: 0; transform: translateY(-3px); }

</style>