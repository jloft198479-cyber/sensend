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
const LS_KEY = 'last-auto-update-check-at'

// 模块级单例：启动静默检查 / 手动检查 / toast 按钮共用同一状态
const status = ref<UpdateStatus>('idle')

let currentUpdate: Update | null = null

/** 统一检查入口：先探测代理（GitHub 直连国内不稳；探测即时或最多 100ms），再调插件 check()。null = 已是最新 */
async function checkUpdateAvailability() {
  await invoke('detect_proxy_for_update').catch(() => {})
  return check({ timeout: 15_000 })
}

export function useUpdater() {
  const { info, error, updateMessage, remove } = useToast()

  /** 启动静默检查：24h 限频；发现新版仅弹 toast 提供「更新」入口，不自动下载 */
  async function silentCheckForUpdate() {
    if (status.value !== 'idle') return
    const last = Number(localStorage.getItem(LS_KEY) || 0)
    if (Date.now() - last < CHECK_INTERVAL_MS) return
    localStorage.setItem(LS_KEY, String(Date.now()))
    try {
      currentUpdate = await checkUpdateAvailability()
      if (currentUpdate) {
        info(`发现新版本 v${currentUpdate.version}`, {
          label: '更新',
          onClick: () => startDownloadInstall(),
        })
      }
    } catch {
      // 静默：网络不通不打扰用户
    }
  }

  /** 手动检查（设置菜单）：发现新版直接进入下载安装 */
  async function manualCheck(beforeInstall?: () => Promise<void>) {
    if (status.value === 'downloading' || status.value === 'installing') return
    status.value = 'checking'
    try {
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
    }
  }

  return { status, silentCheckForUpdate, manualCheck, startDownloadInstall }
}
