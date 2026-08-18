import { ref, reactive, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

import type { PlatformInstance, PlatformTypeInfo } from '../types/platform'

export function useConfig() {
  // ── 状态 ──
  const instances = ref<PlatformInstance[]>([])
  const platformTypes = ref<PlatformTypeInfo[]>([])
  const showForm = ref(false)
  const editingInstanceId = ref<string | null>(null)
  const configForm = reactive({
    name: '',
    platform_type: 'notion',
    token: '',
    token2: '',
    target_id: '',
    publish_mode: 'page',
  })
  const isTesting = ref(false)
  const isSaving = ref(false)
  const testResult = ref<'idle' | 'ok' | 'fail'>('idle')
  const testErrorMsg = ref('')
  const showDeleteConfirm = ref<string | null>(null)
  const showToken = ref(false)
  const savePulse = ref(false)

  // ── 计算属性 ──
  const currentPlatformFields = computed(() => {
    const type = platformTypes.value.find(t => t.key === configForm.platform_type)
    return type?.fields || []
  })

  const canSave = computed(() => {
    if (!configForm.name.trim()) return false
    for (const field of currentPlatformFields.value) {
      if (field.hidden || field.optional) continue
      const val = (configForm as any)[field.key]
      if (!val || !val.trim()) return false
    }
    return true
  })

  // ── 初始化 ──
  async function loadData() {
    try {
      platformTypes.value = await invoke<PlatformTypeInfo[]>('get_platform_types')
    } catch (e) {
      console.error('加载平台类型失败:', e)
    }
    try {
      instances.value = await invoke<PlatformInstance[]>('list_platform_instances')
    } catch (e) {
      console.error('加载平台实例失败:', e)
    }
  }

  // ── 授权码记忆：按平台预填上次填写的 token/token2 ──
  async function applyTokenMemory(platform: string) {
    try {
      const mem = await invoke<{ token: string; token2: string } | null>('get_token_memory', { platformType: platform })
      // 仅在输入为空时回填，避免异步返回时覆盖用户手输内容
      if (!configForm.token) configForm.token = mem?.token || ''
      if (!configForm.token2) configForm.token2 = mem?.token2 || ''
    } catch {
      // 记忆读取失败不阻塞表单
    }
  }

  // ── 表单操作 ──
  function openAddModal() {
    editingInstanceId.value = null
    configForm.name = ''
    configForm.platform_type = 'notion'
    configForm.token = ''
    configForm.token2 = ''
    configForm.target_id = ''
    configForm.publish_mode = 'page'
    testResult.value = 'idle'
    testErrorMsg.value = ''
    showToken.value = false
    showForm.value = true
    // 表单先出现，授权码记忆异步回填，不阻塞切换动画
    void applyTokenMemory(configForm.platform_type)
  }

  function openEditModal(inst: PlatformInstance) {
    editingInstanceId.value = inst.id
    configForm.name = inst.name
    configForm.platform_type = inst.platform_type
    configForm.token = inst.token
    configForm.token2 = inst.token2 || ''
    configForm.target_id = inst.target_id
    configForm.publish_mode = inst.publish_mode || 'page'
    testResult.value = 'ok'
    showForm.value = true
  }

  async function browseFolder() {
    const selected = await open({ directory: true, multiple: false })
    if (selected) configForm.target_id = selected as string
  }

  async function onPlatformTypeChange() {
    testResult.value = 'idle'
    testErrorMsg.value = ''
    configForm.token = ''
    configForm.token2 = ''
    configForm.target_id = ''
    configForm.publish_mode = 'page'
    showToken.value = false
    await applyTokenMemory(configForm.platform_type)
  }

  function resolveField(key: string): string {
    const raw = ((configForm as any)[key] || '').trim()
    if (raw) return raw
    const field = currentPlatformFields.value.find(f => f.key === key)
    return field?.default_value || ''
  }

  async function testConnection() {
    if (!canSave.value) return
    isTesting.value = true
    testResult.value = 'idle'
    testErrorMsg.value = ''
    try {
      const instance: PlatformInstance = {
        id: editingInstanceId.value || crypto.randomUUID(),
        name: configForm.name || configForm.platform_type,
        platform_type: configForm.platform_type,
        token: resolveField('token'),
        token2: resolveField('token2'),
        target_id: resolveField('target_id'),
        publish_mode: configForm.publish_mode,
      }
      await invoke('test_platform_connection', { instance })
      testResult.value = 'ok'
      savePulse.value = true
      setTimeout(() => { savePulse.value = false }, 600)
    } catch (e: any) {
      testResult.value = 'fail'
      testErrorMsg.value = e?.message || String(e) || ''
    } finally {
      isTesting.value = false
    }
  }

  async function saveInstance() {
    if (!canSave.value) return
    // 防重入：保存过程中禁止再次点击
    if (isSaving.value) return
    isSaving.value = true
    try {
      const instance: PlatformInstance = {
        id: editingInstanceId.value || crypto.randomUUID(),
        name: configForm.name.trim(),
        platform_type: configForm.platform_type,
        token: resolveField('token'),
        token2: resolveField('token2'),
        target_id: resolveField('target_id'),
        publish_mode: configForm.publish_mode,
      }
      await invoke('save_platform_instance', { instance })
      // 保存成功即更新该平台授权码记忆（新增/编辑均计入），失败不影响保存
      try {
        await invoke('set_token_memory', {
          platformType: instance.platform_type,
          token: instance.token,
          token2: instance.token2 || '',
        })
      } catch {
        console.warn('授权码记忆更新失败')
      }
      if (editingInstanceId.value) {
        const idx = instances.value.findIndex(i => i.id === editingInstanceId.value)
        if (idx >= 0) instances.value[idx] = instance
      } else {
        instances.value.push(instance)
      }
      editingInstanceId.value = null
      showForm.value = false
    } catch (e: any) {
      console.error('保存失败:', e)
    } finally {
      isSaving.value = false
    }
  }

  async function deleteInstance(id: string) {
    try {
      await invoke('delete_platform_instance', { instanceId: id })
      instances.value = instances.value.filter(i => i.id !== id)
      showDeleteConfirm.value = null
    } catch (e) {
      console.error('删除失败:', e)
    }
  }

  return {
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
    canSave,
    loadData,
    openAddModal,
    openEditModal,
    browseFolder,
    onPlatformTypeChange,
    testConnection,
    saveInstance,
    deleteInstance,
  }
}