import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { PlatformInstance, PlatformTypeInfo, PublishResult } from '../types/platform'
import { useToast } from './useToast'

/**
 * 平台实例管理 + 发送逻辑 composable
 */
export function usePlatform() {
  const { success: toastSuccess, error: toastError } = useToast()
  const instances = ref<PlatformInstance[]>([])
  const activeInstanceId = ref<string | null>(null)
  const platformTypes = ref<PlatformTypeInfo[]>([])
  const isSending = ref(false)

  const activeInstance = computed(() => {
    return instances.value.find(i => i.id === activeInstanceId.value) || null
  })

  /// 从底部栏选择平台 → 更新默认目标
  function selectTarget(instanceId: string) {
    activeInstanceId.value = instanceId
    localStorage.setItem('sensend-default-target', instanceId)
  }

  async function reloadInstances() {
    try {
      instances.value = await invoke<PlatformInstance[]>('list_platform_instances')
      if (instances.value.length > 0) {
        const currentStillExists = instances.value.find(i => i.id === activeInstanceId.value)
        if (!currentStillExists) {
          const localInst = instances.value.find(i => i.platform_type === 'local')
          activeInstanceId.value = localInst?.id || null
        }
      } else {
        activeInstanceId.value = null
      }
    } catch (e) {
      console.error('刷新平台实例失败:', e)
    }
  }

  async function openConfigWindow() {
    try {
      await invoke('open_config_window')
    } catch (e: any) {
      toastError('打开配置窗口失败: ' + (e?.message || String(e)))
    }
  }

  /// 友好化错误信息（401/403/429 等常见状态码 → 人话）
  function friendlyError(raw: string): string {
    if (/401|unauthorized|认证失败/i.test(raw)) return 'Token 过期或无效，请前往配置检查'
    if (/403|forbidden|无权限/i.test(raw)) return '无权限访问目标，请检查 Token 权限'
    if (/429|rate.?limit|频率/i.test(raw)) return '请求过于频繁，请稍后再试'
    if (/network|connect|refused|dns/i.test(raw)) return '网络连接失败，请检查网络'
    return raw
  }

  /// 发送笔记（overrideTargetId 由 App.vue 的 resolvedTarget 提供）
  async function publishNote(editorValue: any, overrideTargetId: string | null) {
    if (!editorValue) return

    const text = editorValue.getText({ blockSeparator: '\n' }).replace(/@\S+/g, '').trim()
    if (!text) {
      toastError('请先输入内容')
      return
    }

    if (!navigator.onLine) {
      toastError('当前无网络连接，请检查网络后重试')
      return
    }

    const targetId = overrideTargetId || activeInstanceId.value
    if (!targetId) {
      openConfigWindow()
      return
    }

    // 剔除 @mention 节点
    function stripMentions(node: any): any {
      const result: any = { type: node.type }
      if (node.attrs) result.attrs = node.attrs
      if (node.content) result.content = node.content.filter((n: any) => n.type !== 'mention').map(stripMentions)
      if (node.marks) result.marks = node.marks
      if (node.text) result.text = node.text
      return result
    }

    isSending.value = true
    try {
      const jsonTree = editorValue.getJSON()
      jsonTree.content = jsonTree.content?.map(stripMentions) ?? []

      const result = await invoke<PublishResult>('publish_note', {
        instanceId: targetId,
        content: jsonTree,
      })

      const url = result.url
      toastSuccess('发送成功', url ? { label: '查看 ↗', onClick: () => openUrl(url) } : undefined)

      // 记忆本次发送目标
      activeInstanceId.value = targetId
      localStorage.setItem('sensend-default-target', targetId)
    } catch (e: any) {
      const raw = e?.message || String(e)
      toastError(friendlyError(raw))
    } finally {
      isSending.value = false
    }
  }

  // ── 启动时加载 ──
  onMounted(async () => {
    try {
      platformTypes.value = await invoke<PlatformTypeInfo[]>('get_platform_types')
    } catch (e) {
      console.error('加载平台类型失败:', e)
    }

    try {
      instances.value = await invoke<PlatformInstance[]>('list_platform_instances')
      if (instances.value.length > 0) {
        const savedId = localStorage.getItem('sensend-default-target')
        const savedExists = savedId && instances.value.find(i => i.id === savedId)
        if (savedExists) {
          activeInstanceId.value = savedId
        } else {
          const localInst = instances.value.find(i => i.platform_type === 'local')
          activeInstanceId.value = localInst?.id || null
        }
      }
    } catch (e) {
      console.error('加载平台实例失败:', e)
    }

    // 监听配置窗口的实例更新事件
    const mainWindow = getCurrentWindow()
    const unlisten = await mainWindow.listen('instances-updated', () => {
      reloadInstances()
    })
    onBeforeUnmount(() => { unlisten() })
  })

  return {
    instances,
    activeInstanceId,
    platformTypes,
    isSending,
    activeInstance,
    selectTarget,
    openConfigWindow,
    publishNote,
    reloadInstances,
  }
}