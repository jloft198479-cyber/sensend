<script setup lang="ts">
import { onMounted } from 'vue'
import { getColorForType } from './types/platform'
import { useConfig } from './composables/useConfig'

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

onMounted(() => { loadData() })
</script>

<template>
  <div class="config-app">
    <!-- ═══ 顶栏 ═══ -->
    <header class="config-titlebar">
      <div class="title-left">
        <span class="title">{{ showForm ? (editingInstanceId ? '编辑页面' : '添加页面') : '平台页面管理' }}</span>
      </div>
      <div class="title-actions">
        <button v-if="showForm" class="title-btn back-title-btn" @click="showForm = false" title="返回列表" aria-label="返回列表">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>
          <span>返回</span>
        </button>
        <button v-else class="title-btn add-new-btn" @click="openAddModal" aria-label="添加页面">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
          </svg>
          <span>添加页面</span>
        </button>
      </div>
    </header>

    <!-- ═══ 编辑表单（新增/编辑模式）═══ -->
    <Transition name="slide">
    <div v-if="showForm" class="edit-section">

      <div class="edit-form">
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
        <!-- 写入模式（local 不支持追加，隐藏） -->
        <div v-if="configForm.platform_type !== 'local' && configForm.platform_type !== 'lark'" class="form-row">
          <label class="form-label">写入方式</label>
          <div class="platform-select">
            <select v-model="configForm.publish_mode">
              <option value="page">创建子页面</option>
              <option value="block">追加到页面</option>
            </select>
          </div>
        </div>
        <template v-for="field in currentPlatformFields" :key="field.key">
          <div v-if="!field.hidden" class="form-row">
            <label class="form-label">
              {{ field.label }}
              <button v-if="field.browse" class="browse-btn" @click="browseFolder" aria-label="浏览文件夹">浏览…</button>
            </label>
            <div class="input-with-toggle">
              <input v-model="(configForm as any)[field.key]"
                :type="field.secret && !showToken ? 'password' : 'text'" class="form-input"
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
        <div class="form-actions">
          <button class="action-btn test-btn" :disabled="!canSave || isTesting"
            @click="testConnection" aria-label="测试连接">
            <svg v-if="isTesting" class="spin-icon" width="13" height="13" viewBox="0 0 24 24" fill="none"
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
    </Transition>

    <!-- ═══ 已配置平台列表（编辑表单展开时隐藏）═══ -->
    <div v-if="!showForm" class="list-section">
      <div v-if="instances.length === 0" class="empty-state">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#d4d4d8"
          stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2" y="3" width="20" height="18" rx="2"/>
          <path d="M12 8v8M8 12h8"/>
        </svg>
        <p>还没有配置任何发送页面</p>
        <p class="empty-hint">点击上方"添加页面"开始配置</p>
      </div>
      <div v-else class="instance-list">
        <div v-for="inst in instances" :key="inst.id" class="instance-card">
          <div class="instance-info">
            <span class="instance-dot" :style="{ background: getColorForType(platformTypes, inst.platform_type) }"></span>
            <div class="instance-text">
              <span class="instance-name">{{ inst.name }}</span>
              <span class="instance-type">
                {{ platformTypes.find(t => t.key === inst.platform_type)?.name }}
                <span v-if="inst.publish_mode === 'block'" class="mode-tag">追加</span>
                <span v-else-if="inst.publish_mode === 'page'" class="mode-tag page-tag">子页面</span>
              </span>
            </div>
          </div>
          <div class="instance-actions">
            <button v-if="showDeleteConfirm === inst.id" class="confirm-btn delete-confirm" @click="deleteInstance(inst.id)" aria-label="确认删除">
              确认删除
            </button>
            <template v-else>
              <button class="icon-btn" @click="openEditModal(inst)" title="编辑" aria-label="编辑">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
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
  padding: 0 20px;
  height: 52px;
  background: var(--bg);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}
.title-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.title {
  font-size: 14px;
  font-weight: 650;
  color: var(--fg);
  letter-spacing: -0.25px;
}
.title-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.title-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 6px 14px;
  border: none;
  border-radius: 8px;
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  font-family: var(--font-sans);
}
.title-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.add-new-btn {
  background: var(--accent);
  color: white;
}
.add-new-btn:hover:not(:disabled) {
  background: var(--accent-hover);
}
.back-title-btn {
  background: var(--gray-hover);
  color: var(--fg-secondary);
}
.back-title-btn:hover {
  background: var(--gray-border);
  color: var(--fg);
}
.edit-section {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  background: var(--bg);
  border-bottom: 1px solid var(--border-color);
}


.edit-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.form-row {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.form-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--fg);
  display: flex;
  align-items: center;
  gap: 6px;
}
.label-hint {
  font-size: 10.5px;
  font-weight: 400;
  color: var(--muted);
  font-style: italic;
}
.field-hint {
  font-size: 11px;
  color: var(--muted);
  margin-top: 2px;
  line-height: 1.4;
}
.browse-btn {
  margin-left: auto;
  padding: 1px 8px;
  border: 1px solid var(--gray-border);
  border-radius: 4px;
  background: var(--bg);
  font-size: 11px;
  color: var(--fg-secondary);
  cursor: pointer;
  transition: all 0.15s;
  font-family: var(--font-sans);
}
.browse-btn:hover { background: var(--gray-hover); border-color: var(--gray-scrollbar); }

.platform-select {
  position: relative;
}
.platform-select select {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--gray-border);
  border-radius: 8px;
  background: var(--gray-input-bg);
  font-size: 13px;
  font-weight: 500;
  color: var(--fg);
  cursor: pointer;
  outline: none;
  appearance: none;
  font-family: var(--font-sans);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2371717a' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 10px center;
  transition: border-color 0.15s;
}
.platform-select select:focus { border-color: var(--accent); background-color: var(--bg); }
.platform-select select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  background-color: var(--gray-hover);
}

.form-input {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--gray-border);
  border-radius: 8px;
  font-size: 13px;
  color: var(--fg);
  background: var(--gray-input-bg);
  outline: none;
  font-family: var(--font-mono);
  transition: border-color 0.15s;
  box-sizing: border-box;
}
.form-input:focus { border-color: var(--accent); background: white; }
.form-input::placeholder { color: var(--gray-placeholder); }

.input-with-toggle {
  position: relative;
  display: flex;
  align-items: center;
}
.input-with-toggle .form-input {
  padding-right: 36px;
}
.toggle-visibility {
  position: absolute;
  right: 4px;
  width: 28px;
  height: 28px;
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
}
.toggle-visibility:hover {
  background: var(--gray-hover);
  color: var(--fg-secondary);
}

.form-actions {
  display: flex;
  gap: 10px;
  margin-top: 2px;
}
.action-btn {
  flex: 1;
  padding: 9px 0;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  font-family: var(--font-sans);
}
.action-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.test-btn { background: var(--gray-hover); color: var(--fg); }
.test-btn:hover:not(:disabled) { background: var(--gray-border); }
.save-btn { background: var(--accent); color: white; }
.save-btn:hover:not(:disabled) { background: var(--accent-hover); }

/* ═══ 实例列表 ═══ */
.list-section {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  color: var(--muted);
  text-align: center;
}
.empty-state p {
  margin: 0;
  font-size: 14px;
  font-weight: 500;
}
.empty-hint {
  margin-top: 6px !important;
  font-size: 12px !important;
  font-weight: 400 !important;
  color: var(--gray-placeholder);
}

.instance-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.instance-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  background: var(--bg);
  border: 1px solid var(--border-color);
  border-radius: 10px;
  transition: all 0.15s ease;
}
.instance-card:hover {
  border-color: var(--gray-border);
  box-shadow: 0 1px 3px rgba(0,0,0,0.04);
}
.instance-info {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.instance-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.instance-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.instance-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--fg);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.instance-type {
  font-size: 11px;
  color: var(--muted);
  display: flex;
  align-items: center;
  gap: 5px;
}
.mode-tag {
  font-size: 10px;
  font-weight: 600;
  color: var(--accent);
  background: rgba(44, 175, 104, 0.1);
  padding: 0 5px;
  border-radius: 3px;
  line-height: 16px;
}
.mode-tag.page-tag {
  color: var(--accent);
  background: rgba(44, 175, 104, 0.1);
}
.instance-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.icon-btn {
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
  transition: all 0.15s ease;
  padding: 0;
}
.icon-btn:hover { background: var(--gray-hover); color: var(--fg); }
.icon-btn.danger:hover { background: var(--danger-bg); color: var(--danger); }
.confirm-btn {
  padding: 4px 10px;
  border: 1px solid var(--danger);
  background: var(--danger-bg);
  color: var(--danger);
  border-radius: 6px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  font-family: var(--font-sans);
  transition: all 0.15s;
}
.confirm-btn:hover { background: #fee2e2; }

/* ═══ 动画 ═══ */

.slide-enter-active { transition: all 0.2s ease-out; }
.slide-leave-active { transition: all 0.15s ease-in; }
.slide-enter-from { opacity: 0; transform: translateY(-8px); }
.slide-leave-to { opacity: 0; transform: translateY(-4px); }

</style>