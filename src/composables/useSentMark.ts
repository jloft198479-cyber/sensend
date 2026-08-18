// ═══ B7: 发送成功视觉标记 ═══
// 用 ProseMirror Plugin decoration 实现"✓ 已发送"标记
// 红线：不碰文档 JSON——decoration 是纯视觉叠加层

import { Extension } from '@tiptap/core'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'

const sentMarkKey = new PluginKey<DecorationSet | null>('sentMark')

/// TipTap 扩展：发送成功后在文档末尾显示"✓ 已发送"标记
export const SentMarkExtension = Extension.create({
  name: 'sentMark',

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet | null>({
        key: sentMarkKey,
        state: {
          init() { return null },
          apply(tr, old) {
            const meta = tr.getMeta(sentMarkKey)
            if (meta === 'sent') {
              // 在文档末尾创建 widget decoration
              const pos = tr.doc.content.size
              const widget = Decoration.widget(pos, () => {
                const el = document.createElement('span')
                el.className = 'sent-mark-badge'
                el.textContent = '\u2713 \u5df2\u53d1\u9001'
                return el
              }, { side: 1 })
              return DecorationSet.create(tr.doc, [widget])
            }
            if (meta === 'clear') return null
            // 文档变化时映射 decoration 位置
            return old ? old.map(tr.mapping, tr.doc) : null
          },
        },
        props: {
          decorations(state) {
            return this.getState(state)
          },
        },
      }),
    ]
  },
})

/// 触发"已发送"标记，3 秒后自动消失
export function markSent(editor: any) {
  if (!editor) return
  const tr = editor.state.tr.setMeta(sentMarkKey, 'sent')
  tr.setMeta('addToHistory', false)
  editor.view.dispatch(tr)
  // 3 秒后清除
  setTimeout(() => {
    if (!editor.isDestroyed) {
      const clearTr = editor.state.tr.setMeta(sentMarkKey, 'clear')
      clearTr.setMeta('addToHistory', false)
      editor.view.dispatch(clearTr)
    }
  }, 3000)
}
