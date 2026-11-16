import { ref, reactive, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

import type { PlatformInstance, PlatformTypeInfo } from '../types/platform'
import { useToast } from './useToast'

// 新建实例的默认平台类型 key（与 Rust 侧 get_platform_types 对应）
const DEFAULT_PLATFORM_TYPE = 'notion'

export function useConfig() {
  // ── 状态 ──
  const instances = ref<PlatformInstance[]>([])
  const platformTypes = ref<PlatformTypeInfo[]>([])
  // [ZC-03] 首次加载未落地前列表区渲染骨架，而不是"还没有配置任何发送页面"
  // （空数组初值会被模板误读成"没有配置"——每次打开都先说一句假话的根源）
  const loaded = ref(false)
  // [ZC-10] 当前默认发送目标（instance id，null=未设置）。管理页此前唯一管不了的事
  const defaultTargetId = ref<string | null>(null)
  const showForm = ref(false)
  const editingInstanceId = ref<string | null>(null)
  const configForm = reactive({
    name: '',
    platform_type: DEFAULT_PLATFORM_TYPE,
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

  // 当前平台的写入模式列表（SSOT：后端 get_platform_types.supported_modes 下发）。
  // 元数据缺失时兜底两种都有，保持旧版 notion/flowus 行为
  const currentModes = computed<string[]>(() => {
    const modes = platformTypes.value.find(t => t.key === configForm.platform_type)?.supported_modes
    return modes && modes.length > 0 ? modes : ['page', 'block']
  })

  /** 把 publish_mode 归一到当前平台支持的档位（切平台/载入实例共用） */
  function normalizeMode(mode: string): string {
    return currentModes.value.includes(mode) ? mode : currentModes.value[0]
  }

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
  // [m-27] 两个 IPC 无依赖，并行发起（与 usePlatform 启动一致）；
  // 失败必须让用户看见——静默只 console.error 会让配置窗口显示空列表而毫无解释。
  async function loadData() {
    try {
      const [typesRes, listRes, targetRes] = await Promise.allSettled([
        invoke<PlatformTypeInfo[]>('get_platform_types'),
        invoke<PlatformInstance[]>('list_platform_instances'),
        invoke<string | null>('get_default_target'),
      ])
      if (typesRes.status === 'fulfilled') {
        platformTypes.value = typesRes.value
      } else {
        console.error('加载平台类型失败:', typesRes.reason)
        useToast().error(`加载平台类型失败：${(typesRes.reason as any)?.message ?? typesRes.reason}`)
      }
      if (listRes.status === 'fulfilled') {
        instances.value = listRes.value
      } else {
        console.error('加载平台实例失败:', listRes.reason)
        useToast().error(`加载页面列表失败：${(listRes.reason as any)?.message ?? listRes.reason}`)
      }
      if (targetRes.status === 'fulfilled') {
        defaultTargetId.value = targetRes.value
      } else {
        // [ZC-10] 读取失败按"未设置"呈现（不谎报），失败原因打日志备查
        console.error('读取默认发送目标失败:', targetRes.reason)
        defaultTargetId.value = null
      }
    } finally {
      // [ZC-03] 成败都算"已加载"（失败已 toast），骨架让位给真实状态
      loaded.value = true
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
    configForm.platform_type = DEFAULT_PLATFORM_TYPE
    configForm.token = ''
    configForm.token2 = ''
    configForm.target_id = ''
    configForm.publish_mode = normalizeMode('page')
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
    configForm.publish_mode = normalizeMode(inst.publish_mode || 'page')
    // [m-27] 这三项必须跟「新建」一样重置，否则会从上一次操作残留：
    // testResult 之前直接置 'ok' → 明明没测过却显示「连接成功，可以保存了」；
    // showToken 残留会让密钥以明文呈现（编辑窗口默认必须是打码态）。
    testResult.value = 'idle'
    testErrorMsg.value = ''
    showToken.value = false
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
    configForm.publish_mode = normalizeMode('page')
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
      // 失败必须可见：静默会让用户误以为已保存（DPAPI 加密失败/存储写入失败都在此路径）
      useToast().error(`保存失败：${e?.message ?? e}`)
    } finally {
      isSaving.value = false
    }
  }

  async function deleteInstance(id: string) {
    try {
      await invoke('delete_platform_instance', { instanceId: id })
      instances.value = instances.value.filter(i => i.id !== id)
      // [ZC-10] 删掉的若正是默认目标，本地同步清掉，避免"当前目标"卡指向幽灵
      if (defaultTargetId.value === id) defaultTargetId.value = null
      showDeleteConfirm.value = null
    } catch (e) {
      console.error('删除失败:', e)
      useToast().error(`删除失败：${(e as any)?.message ?? e}`)
    }
  }

  /** [ZC-10] 设为默认发送目标。后端 set_default_target 会 emit instances-updated，
   *  主窗监听后 reloadInstances 重读 default_target → 底栏当前目标随之同步 */
  async function setAsDefault(id: string) {
    if (defaultTargetId.value === id) return
    try {
      await invoke('set_default_target', { targetId: id })
      defaultTargetId.value = id
      const name = instances.value.find(i => i.id === id)?.name || ''
      useToast().success(`已设为默认发送目标：${name}`)
    } catch (e) {
      console.error('设置默认目标失败:', e)
      useToast().error(`设置默认目标失败：${(e as any)?.message ?? e}`)
    }
  }

  return {
    instances,
    platformTypes,
    loaded,
    defaultTargetId,
    setAsDefault,
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