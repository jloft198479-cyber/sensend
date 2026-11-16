import { ref } from 'vue'
import { check } from '@tauri-apps/plugin-updater'
import type { Update } from '@tauri-apps/plugin-updater'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from './useToast'

/**
 * 应用内自动更新（设计：docs/AUTO-UPDATE-DESIGN.md）
 * 更新源在 tauri.conf.json endpoints 按序尝试；当前 GitHub 直连 + 代理探测兜底（solo 同款），
 * CNB 仓库就绪后把直连端点移到数组首位即可，本模块无需改动。
 * 检查：启动静默（24h 限频，失败不打扰）+ 设置菜单手动；安装前经 beforeInstall 善后（保存笔记）。
 */

export type UpdateStatus = 'idle' | 'checking' | 'downloading' | 'installing' | 'error'

const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000
/** [m-17] 检查失败后的退避：旧实现「先记时间戳再检查」，网络一断就把 24 小时限频额度浪费掉，
 *  用户当天再也等不到静默检查。失败只退避 1 小时，既不骚扰也不至于整天不重试。 */
const FAIL_RETRY_MS = 60 * 60 * 1000
const LS_KEY = 'last-auto-update-check-at'

/** 读取上次检查记录；兼容旧版存的纯数字时间戳 */
function readLastCheck(): { at: number; ok: boolean } {
  try {
    const raw = localStorage.getItem(LS_KEY)
    if (!raw) return { at: 0, ok: true }
    if (raw.startsWith('{')) {
      const v = JSON.parse(raw) as { at?: number; ok?: boolean }
      return { at: Number(v.at) || 0, ok: v.ok !== false }
    }
    const at = Number(raw)
    return { at: Number.isFinite(at) ? at : 0, ok: true }
  } catch {
    return { at: 0, ok: true }
  }
}

function writeLastCheck(ok: boolean) {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify({ at: Date.now(), ok }))
  } catch {
    // 隐私模式等写不进去：退化成每次都检查，不影响功能
  }
}

// 模块级单例：启动静默检查 / 手动检查 / toast 按钮共用同一状态
const status = ref<UpdateStatus>('idle')

let currentUpdate: Update | null = null

/** 统一检查入口：先探测代理（GitHub 直连国内不稳），再调插件 check()。null = 已是最新
    探测耗时：环境变量/系统代理命中即返回；回落到本地端口探针时最坏约 3 秒（两端口 × 连接+写+读） */
async function checkUpdateAvailability() {
  await invoke('detect_proxy_for_update').catch(() => {})
  return check({ timeout: 15_000 })
}

/** [m-13 修复] 更新流程收尾时把 HTTPS_PROXY 还原成用户原值。
    后端幂等：从没设过就什么都不做。代理必须在 download() 期间仍然有效，
    所以只在「确认没有新版本」和「下载安装结束」时调用。 */
function restoreProxyEnv() {
  return invoke('restore_update_proxy').catch(() => {})
}

export function useUpdater() {
  const { info, error, updateMessage, remove } = useToast()

  /** 启动静默检查：24h 限频；发现新版弹常驻 toast 提供「更新」入口（2.5s 会点不到），不自动下载。
      beforeInstall 透传给下载安装链（缺了 = 装机丢笔记，App.vue 必须传 flushSave） */
  async function silentCheckForUpdate(beforeInstall?: () => Promise<void>) {
    if (status.value !== 'idle') return
    const last = readLastCheck()
    if (last.at && Date.now() - last.at < (last.ok ? CHECK_INTERVAL_MS : FAIL_RETRY_MS)) return
    status.value = 'checking'
    try {
      // 先置 null 再关：check 抛错时旧引用不残留（旧 toast 点击会被 startDownloadInstall 安全短路）
      const stale = currentUpdate
      currentUpdate = null
      stale?.close().catch(() => {})
      currentUpdate = await checkUpdateAvailability()
      writeLastCheck(true)   // 真的问到结果才算「检查过」，失败不计入 24h 额度
      if (currentUpdate) {
        info(`发现新版本 v${currentUpdate.version}`, 0, {
          label: '更新',
          onClick: () => startDownloadInstall(beforeInstall),
        })
      }
    } catch {
      // 静默：网络不通不打扰用户，但只退避 1 小时，下次启动还会再试
      writeLastCheck(false)
    } finally {
      if (status.value === 'checking') status.value = 'idle'
      // 没发现新版本 = 本次更新流程结束，还原用户的 HTTPS_PROXY；
      // 有更新则留着，download() 还需要它
      if (!currentUpdate) await restoreProxyEnv()
    }
  }

  /** 手动检查（设置菜单）：发现新版直接进入下载安装 */
  async function manualCheck(beforeInstall?: () => Promise<void>) {
    // [m-17] 忙时不能静默 return：用户点了「检查更新」什么都没发生，会以为按钮坏了
    if (status.value !== 'idle' && status.value !== 'error') {
      info(status.value === 'checking' ? '正在检查更新，请稍候…' : '更新正在下载安装中…')
      return
    }
    status.value = 'checking'
    try {
      const stale = currentUpdate
      currentUpdate = null
      stale?.close().catch(() => {})
      currentUpdate = await checkUpdateAvailability()
      if (currentUpdate) {
        await startDownloadInstall(beforeInstall)
      } else {
        info('当前已是最新版本')
        status.value = 'idle'
      }
    } catch (e) {
      error(`检查更新失败：${String(e).slice(0, 120)}`)
      status.value = 'error'
    } finally {
      // 下载安装已在上面的 await 里走完（startDownloadInstall 自己也会还原，幂等）
      await restoreProxyEnv()
    }
  }

  /** 下载并安装（toast 常驻进度）；beforeInstall 在启动安装器前执行（保存笔记等善后） */
  async function startDownloadInstall(beforeInstall?: () => Promise<void>) {
    if (!currentUpdate || status.value === 'downloading' || status.value === 'installing') return
    status.value = 'downloading'
    const tid = info('开始下载更新…', 0) // 常驻条目，完成/失败时手动移除
    let total = 0
    let downloaded = 0
    let lastPct = -1
    try {
      await currentUpdate.download((event) => {
        if (event.event === 'Started') {
          total = event.data.contentLength ?? 0
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength
          const pct = total > 0 ? Math.floor((downloaded / total) * 100) : 0
          if (pct !== lastPct) {
            lastPct = pct
            updateMessage(tid, `下载更新中 ${pct}%`)
          }
        }
      })
      updateMessage(tid, '下载完成，正在准备安装…')
      status.value = 'installing'
      // 安装器会直接退出应用（不走 app-exit-request 事件链），善后必须在此前完成
      await beforeInstall?.()
      await currentUpdate.install()
      remove(tid)
    } catch (e) {
      remove(tid)
      error(`更新失败：${String(e).slice(0, 120)}`)
      status.value = 'error'
    } finally {
      // 下载/安装流程结束（成功或失败）→ 还原用户的 HTTPS_PROXY
      await restoreProxyEnv()
    }
  }

  return { status, silentCheckForUpdate, manualCheck, startDownloadInstall }
}
