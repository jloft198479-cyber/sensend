<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { useConfig } from './composables/useConfig'
import TargetIcon from './components/TargetIcon.vue'
import ToastLayer from './components/ToastLayer.vue'

const {
  instances,
  platformTypes,
  showForm,
  editingInstanceId,
  configForm,
  isTesting,
  isSaving,
  testResult,
  testErrorMsg,
  showDeleteConfirm,
  showToken,
  savePulse,
  currentPlatformFields,
  currentModes,
  canSave,
  isDirty,
  loaded,
  defaultTargetId,
  setAsDefault,
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

// [ZC-10] 当前默认目标的实例对象（null = 未设置 / 指向的实例已被删除）
const currentDefaultTarget = computed(() =>
  defaultTargetId.value
    ? instances.value.find(i => i.id === defaultTargetId.value) ?? null
    : null,
)
const dtTypeName = computed(() =>
  currentDefaultTarget.value
    ? platformTypes.value.find(t => t.key === currentDefaultTarget.value!.platform_type)?.name
      || currentDefaultTarget.value.platform_type
    : '',
)

// [ZC-12] 删除确认改"并列 + 取消"：确认/取消与编辑删除图标并排，用户自主决定，
// 不再 3 秒自动回退（自动回退会让手慢的用户没点完就丢掉确认态）
// [ZC-12] 返回/关闭的 dirty 拦截：行内二次确认（同按钮变文案），不弹模态框
const exitConfirm = ref<'none' | 'back' | 'close'>('none')
let exitTimer: ReturnType<typeof setTimeout> | null = null
function doExit(kind: 'back' | 'close') {
  exitConfirm.value = 'none'
  if (exitTimer) { clearTimeout(exitTimer); exitTimer = null }
  if (kind === 'back') showForm.value = false
  else closeWindow()
}
function requestExit(kind: 'back' | 'close') {
  // 无改动直接走；第一次点进入确认态（3 秒不点自动回退是辅助，取消/再点是主路径）；
  // 第二次点同一按钮 = 确认放弃
  if (!isDirty.value || exitConfirm.value === kind) { doExit(kind); return }
  exitConfirm.value = kind
  if (exitTimer) clearTimeout(exitTimer)
  exitTimer = setTimeout(() => { exitConfirm.value = 'none' }, 3000)
}
onBeforeUnmount(() => { if (exitTimer) clearTimeout(exitTimer) })

onMounted(() => {
  loadData()
  // [ZC-02] 后端建窗为隐藏态（.visible(false) + builder 阶段已上底色）：
  // 此刻骨架/列表首帧 DOM 已就绪，主动 show，全程无白窗、无假空态。
  // 浏览器直开（无 Tauri 宿主）时 IPC 会 reject，静默忽略。
  getCurrentWindow().show().catch(() => {})
  // [ZC-01 配套] 关闭=隐藏后窗口不重新挂载，每次重新显示时重拉一次数据，
  // 避免隐藏期间别处改动（主题/实例）导致管理窗展示旧状态
  void listen('config-window-shown', () => { loadData() }).catch(() => {})
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
      <!-- [ZC-12] 窗口标题：此前顶栏没有一个字说明这是什么窗口 -->
      <span class="config-title">发送目标</span>
      <div class="title-actions">
        <button v-if="showForm" class="title-btn back-title-btn"
          :class="{ 'confirm-exit': exitConfirm === 'back' && isDirty }"
          @click="requestExit('back')" :title="exitConfirm === 'back' && isDirty ? '再次点击放弃修改' : '返回列表'"
          :aria-label="exitConfirm === 'back' && isDirty ? '确认放弃修改并返回' : '返回列表'">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          <span>{{ exitConfirm === 'back' && isDirty ? '确认放弃？' : '返回' }}</span>
        </button>
        <button v-else class="title-btn add-new-btn" @click="openAddModal" aria-label="新增页面">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          <span>新增页面</span>
        </button>
        <button class="close-win-btn" :class="{ 'confirm-exit': exitConfirm === 'close' && isDirty }"
          @click="requestExit('close')" :title="exitConfirm === 'close' && isDirty ? '再次点击放弃修改并关闭' : '关闭'"
          :aria-label="exitConfirm === 'close' && isDirty ? '确认放弃修改并关闭' : '关闭'">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    </header>

    <!-- ═══ 视图切换：编辑表单 / 页面列表（[ZC-12] 瞬时切换——原 out-in 交叉期有
         约 200ms "什么都没有"的空白帧，评审点名。v-if/v-else 互斥，无中间帧） ═══ -->
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
          <button class="action-btn save-btn" :class="{ 'btn-bounce': savePulse }"
            :disabled="!canSave || isSaving" @click="saveInstance" aria-label="保存">
            <!-- [ZC-12] 保存中反馈：落盘+加密+写记忆都在这一步，此前按钮一动不动 -->
            {{ isSaving ? '保存中…' : (editingInstanceId ? '更新' : '保存') }}
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
      <!-- [ZC-10] 当前目标卡：管理页此前唯一管不了的事就是默认目标。loaded 后才渲染，
           骨架期不出现（否则又会像 ZC-03 之前那样先说一句假话） -->
      <div v-if="loaded" class="default-target-card" :class="{ 'is-unset': !currentDefaultTarget }">
        <template v-if="currentDefaultTarget">
          <!-- [ZC-13] 图标走 TargetIcon 单一渲染点，类型用全局 .type-badge -->
          <TargetIcon :platform-type="currentDefaultTarget.platform_type" />
          <div class="dt-text">
            <span class="dt-name">{{ currentDefaultTarget.name }}</span>
            <span class="dt-meta">
              当前目标
              <span class="type-badge">{{ dtTypeName }}</span>
            </span>
          </div>
        </template>
        <template v-else>
          <span class="dt-empty">未设置默认目标，发送时按下方列表首个可用目标</span>
        </template>
      </div>
      <!-- [ZC-03] 加载中：骨架占位（卡片几何与真实条目一致，数据到来不跳动）；不许在数据未落地前说"还没有配置" -->
      <div v-if="!loaded" class="instance-list" aria-hidden="true">
        <div v-for="i in 3" :key="i" class="instance-card skeleton-card">
          <span class="skeleton-line w-name"></span>
          <span class="skeleton-line w-meta"></span>
        </div>
      </div>
      <div v-else-if="instances.length === 0" class="empty-state">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--gray-scrollbar)"
          stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="3" width="20" height="18" rx="2"/>
          <path d="M12 8v8M8 12h8"/>
        </svg>
        <p>还没有配置任何发送页面</p>
        <p class="empty-hint">点击下方按钮，或右上角「新增页面」</p>
        <!-- [ZC-12] 空态给真实动线：文案让用户去点顶栏 40px 高的按钮，不如直接给按钮 -->
        <button class="empty-action-btn" @click="openAddModal">添加第一个目标</button>
      </div>
      <div v-else class="instance-list">
        <!-- [ZC-12] 整卡可点进编辑（此前卡片看着能点、点正文毫无反应）；键盘 Enter/Space 同样可达 -->
        <div v-for="inst in instances" :key="inst.id" class="instance-card is-clickable"
          role="button" tabindex="0" :aria-label="`编辑 ${inst.name}`"
          @click="openEditModal(inst)"
          @keydown.enter.prevent="openEditModal(inst)"
          @keydown.space.prevent="openEditModal(inst)">
          <div class="instance-info">
            <!-- [ZC-13] 列表卡补图标 + 类型徽标：与 @ 列表/底栏同一套显示规则 -->
            <TargetIcon :platform-type="inst.platform_type" />
            <div class="instance-text">
              <span class="instance-name">{{ inst.name }}</span>
              <span class="instance-type">
                <span class="type-badge">{{ platformTypes.find(t => t.key === inst.platform_type)?.name }}</span>
                <span v-if="inst.publish_mode === 'block'" class="mode-tag">追加</span>
                <span v-else-if="inst.publish_mode === 'page'" class="mode-tag">子页面</span>
              </span>
            </div>
          </div>
          <div class="instance-actions">
            <!-- [ZC-12] 删除确认与常态并列（不再顶掉编辑/删除），且给"取消"；行内操作全部 .stop 防冒泡进卡 -->
            <button v-if="showDeleteConfirm === inst.id" class="confirm-btn delete-confirm" @click.stop="deleteInstance(inst.id)" aria-label="确认删除">
              确认删除
            </button>
            <button v-if="showDeleteConfirm === inst.id" class="cancel-delete" @click.stop="showDeleteConfirm = null" aria-label="取消删除">
              取消
            </button>
            <button v-if="defaultTargetId !== inst.id" class="set-default-btn"
              @click.stop="setAsDefault(inst.id)" title="设为默认发送目标" aria-label="设为默认发送目标">
              设为当前
            </button>
            <button class="icon-btn" @click.stop="openEditModal(inst)" title="编辑" aria-label="编辑">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
                <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
              </svg>
            </button>
            <button class="icon-btn danger" :disabled="showDeleteConfirm === inst.id"
              @click.stop="showDeleteConfirm = inst.id" title="删除" aria-label="删除">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 6 5 6 21 6"/>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
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
  justify-content: space-between;
  padding: 0 var(--space-14) 0 var(--space-16);
  height: var(--titlebar-h);
  background: var(--accent-light);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
  cursor: default;
  user-select: none;
}
/* [ZC-12] 窗口标题：管理窗顶栏此前没有一个字说明这是什么窗口 */
.config-title {
  font-size: var(--text-sm);
  font-weight: 700;
  color: var(--fg);
  letter-spacing: 0.01em;
}
.title-actions {
  display: flex;
  align-items: center;
  gap: var(--space-8);
}
/* [ZC-12] 返回/关闭的 dirty 确认态：危险强调（危险确认统一交互，同删除确认） */
.title-btn.confirm-exit,
.close-win-btn.confirm-exit {
  background: var(--danger-bg);
  color: var(--danger);
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
/* [ZC-12] 空态主行动按钮：文案之外给真实动线（与顶栏"新增页面"同规格配色） */
.empty-action-btn {
  margin-top: var(--space-12);
  height: var(--control-h-sm);
  padding: 0 var(--space-16);
  border: none;
  border-radius: var(--radius-md);
  background: var(--accent-fill);
  color: var(--fg-on-accent);
  font-family: var(--font-sans);
  font-size: var(--text-xs);
  font-weight: 600;
  cursor: pointer;
  transition: background var(--dur-fast) ease;
}
.empty-action-btn:hover { background: var(--accent-fill-hover); }
/* [ZC-12] 删除确认的"取消"：与"确认删除"并列的退出路径 */
.cancel-delete {
  height: var(--control-h-xs);
  padding: 0 var(--space-8);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: transparent;
  font-family: var(--font-sans);
  font-size: var(--text-2xs);
  font-weight: 500;
  color: var(--muted);
  cursor: pointer;
  transition: all var(--dur-fast) ease;
}
.cancel-delete:hover { border-color: var(--muted); color: var(--fg); }

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
/* [ZC-12] 整卡可点进编辑：cursor 跟手 + 键盘可达（Enter/Space 由模板绑定，焦点环走全局 :focus-visible） */
.instance-card.is-clickable {
  cursor: pointer;
}
.instance-card:hover {
  border-color: var(--control-border);
  box-shadow: var(--shadow-e1);
}

/* [ZC-10] 当前目标卡：默认目标从"藏在底栏"提升到管理中心的一等公民。
   图标规格与 InstanceMenu.vue 的 .inst-icon 同源（字形规则已在 types/platform.ts
   单一化；样式层收敛等 B3 ZC-13 的共享组件） */
.default-target-card {
  display: flex;
  align-items: center;
  gap: var(--space-10);
  padding: var(--space-10) var(--space-12);
  background: var(--accent-light);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-12);
}
.default-target-card.is-unset { background: var(--bg); }
.dt-text {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}
.dt-name {
  font-size: var(--text-sm);
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dt-meta {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  font-size: var(--text-2xs);
  color: var(--muted);
}
.dt-empty {
  font-size: var(--text-xs);
  color: var(--muted);
}
/* [ZC-10] 行内"设为当前"：次要按钮，hover 才亮出目标色（与删除红的 hover 逻辑对齐） */
.set-default-btn {
  height: var(--control-h-xs);
  padding: 0 var(--space-8);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  background: transparent;
  font-family: var(--font-sans);
  font-size: var(--text-2xs);
  font-weight: 500;
  color: var(--muted);
  cursor: pointer;
  transition: all var(--dur-fast) ease;
}
.set-default-btn:hover {
  color: var(--accent-strong);
  border-color: var(--accent-strong);
}

/* [ZC-03] 加载骨架：复用 .instance-card 几何（同 padding/间距），只把内容换成呼吸占位条；
   占位条高度 = 真实行字高（--text-md / --text-xs），数据到来时不会跳动 */
.skeleton-card {
  flex-direction: column;
  align-items: stretch;
  justify-content: center;
  gap: var(--space-8);
  cursor: default;
}
.skeleton-card:hover {
  border-color: var(--border-color);
  box-shadow: none;
}
.skeleton-line {
  display: block;
  height: var(--text-md);
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  animation: skeleton-breathe 1.4s ease-in-out infinite;
}
.skeleton-line.w-meta { height: var(--text-xs); width: 26%; }
.skeleton-line.w-name { width: 42%; }
@keyframes skeleton-breathe {
  0%, 100% { opacity: 0.55; }
  50% { opacity: 1; }
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
/* [ZC-12] 删除图标默认即危险色（此前红只在 hover 出现，破坏性入口不可隐身） */
.icon-btn.danger { color: var(--danger); opacity: 0.72; }
.icon-btn.danger:hover { opacity: 1; background: var(--danger-bg); }
.icon-btn.danger:disabled { opacity: 0.3; cursor: default; }
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

/* [ZC-12] 视图切换改瞬时（原 out-in 交叉期约 200ms 空白帧），过渡 CSS 一并移除 */

</style>