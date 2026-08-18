// ═══ 优化方案 1b：并行化启动 IPC ═══
// 改动说明（相对原版，仅 [OPT] 标记处有变化）：
// 1. [OPT] onMounted 中 get_platform_types 和 list_platform_instances 从串行 await
//    改为并行发起（.then().catch() 链），两个 IPC 调用同时发出，各自独立处理错误
// 2. 所有其他逻辑（reloadInstances、publishNote、friendlyError 等）完全不变
// 3. 导出 API 完全不变
//
// 收益：消除 usePlatform 内部 1 次串行 IPC 等待（约 20-50ms）

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

  /// 从底部栏选择平台 → 更新默认目标（存后端 config.json，比 localStorage 可靠）
  function selectTarget(instanceId: string) {
    activeInstanceId.value = instanceId
    // fire-and-forget：保持同步签名，不阻塞调用方
    invoke('set_default_target', { targetId: instanceId }).catch(e => {
      console.error('保存默认目标失败:', e)
    })
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
  /// 返回 true 表示发送成功
  async function publishNote(editorValue: any, overrideTargetId: string | null): Promise<boolean> {
    if (!editorValue) return false

    const text = editorValue.getText({ blockSeparator: '\n' }).replace(/(?<!\S)@\S+/g, '').trim()
    if (!text) {
      toastError('请先输入内容')
      return false
    }

    if (!navigator.onLine) {
      toastError('当前无网络连接，请检查网络后重试')
      return false
    }

    const targetId = overrideTargetId || activeInstanceId.value
    if (!targetId) {
      openConfigWindow()
      return false
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
      invoke('set_default_target', { targetId }).catch(e => {
        console.error('保存默认目标失败:', e)
      })
      return true
    } catch (e: any) {
      const raw = e?.message || String(e)
      toastError(friendlyError(raw))
      return false
    } finally {
      isSending.value = false
    }
  }

  // ── 启动时加载 ──
  onMounted(async () => {
    // [OPT] 并行发起两个独立 IPC 调用，不再串行等待
    // 两个 invoke 同时发出（Promise 立即执行），各自 .then() 独立处理结果
    invoke<PlatformTypeInfo[]>('get_platform_types')
      .then(types => { platformTypes.value = types })
      .catch(e => { console.error('加载平台类型失败:', e) })

    invoke<PlatformInstance[]>('list_platform_instances')
      .then(async (list) => {
        instances.value = list
        if (list.length > 0) {
          // 从后端 config.json 读取默认目标，localStorage 作为旧版迁移兜底
          let savedId: string | null = null
          try {
            savedId = await invoke<string | null>('get_default_target')
          } catch {
            savedId = localStorage.getItem('sensend-default-target')
          }
          const savedExists = savedId && list.find(i => i.id === savedId)
          if (savedExists) {
            activeInstanceId.value = savedId
          } else {
            const localInst = list.find(i => i.platform_type === 'local')
            activeInstanceId.value = localInst?.id || null
          }
        }
      })
      .catch(e => { console.error('加载平台实例失败:', e) })

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
