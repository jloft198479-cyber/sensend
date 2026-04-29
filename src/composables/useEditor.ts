import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useEditor as useTiptapEditor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import Mention from '@tiptap/extension-mention'
import { VueRenderer } from '@tiptap/vue-3'
import tippy from 'tippy.js'
import { invoke } from '@tauri-apps/api/core'
import { TextSelection } from '@tiptap/pm/state'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { Ref } from 'vue'
import type { PlatformInstance, PlatformTypeInfo } from '../types/platform'
import { getInstanceDisplayName } from '../types/platform'
import MentionList from '../components/MentionList.vue'

/**
 * 编辑器核心 composable
 * 职责：TipTap 初始化、自动保存、字数统计、@mention
 * 格式操作 → useEditorFormat，字体管理 → useEditorFont
 */
export function useSensendEditor(
  instances: Ref<PlatformInstance[]>,
  platformTypes: Ref<PlatformTypeInfo[]>,
) {
  type SaveStatus = 'idle' | 'saved' | 'saving' | 'unsaved'
  const saveStatus = ref<SaveStatus>('idle')
  const wordCount = ref(0)
  const charCount = ref(0)

  // ── @mention 工具函数 ──

  /** 获取编辑器中 mention 节点的 instance ID（唯一） */
  function getMentionId(): string | null {
    if (!editor.value) return null
    let foundId: string | null = null
    editor.value.state.doc.descendants((node: any) => {
      if (node.type.name === 'mention' && node.attrs.id) {
        foundId = node.attrs.id
      }
    })
    return foundId
  }

  /** 删除编辑器中所有 mention 节点，返回是否实际删除了 */
  function deleteAllMentions(): boolean {
    if (!editor.value) return false
    const { tr } = editor.value.state
    const ranges: Array<[number, number]> = []
    editor.value.state.doc.descendants((node: any, pos: number) => {
      if (node.type.name === 'mention') ranges.push([pos, pos + node.nodeSize])
    })
    if (ranges.length === 0) return false
    for (let i = ranges.length - 1; i >= 0; i--) {
      tr.delete(ranges[i][0], ranges[i][1])
    }
    tr.setMeta('mentionReplace', true)
    editor.value.view.dispatch(tr)
    return true
  }

  /** 从底部栏选择时：清除旧 mention + 插入新 mention */
  function setMention(instanceId: string) {
    if (!editor.value) return
    const inst = instances.value.find(i => i.id === instanceId)
    if (!inst) return

    deleteAllMentions()

    // 在文档开头插入 @mention + 空格
    editor.value.chain()
      .focus()
      .insertContentAt(0, {
        type: 'mention',
        attrs: {
          id: inst.id,
          label: getInstanceDisplayName(platformTypes.value, inst),
          mentionSuggestionChar: '@',
        },
      })
      .insertContentAt(1, ' ')
      .run()
  }

  /** mention 变化时的回调（由外部 App.vue 设置） */
  let onMentionChange: ((mentionId: string | null) => void) | null = null

  /** 注册 mention 变化回调 */
  function setOnMentionChange(cb: (mentionId: string | null) => void) {
    onMentionChange = cb
  }

  // ── 编辑器 ──
  const editor = useTiptapEditor({
    content: '',
    extensions: [
      StarterKit,
      Placeholder.configure({
        placeholder: '开始记录，默认发送到上次位置，@可随时切换',
        emptyEditorClass: 'is-editor-empty',
      }),
      Mention.configure({
        HTMLAttributes: { class: 'platform-mention' },
        renderHTML(props) {
          return ['span', { class: 'platform-mention' }, `@${props.node.attrs.label || props.node.attrs.id}`]
        },
        suggestion: {
          char: '@',
          // 插入新 mention 前先删旧的，保证唯一性
          command: ({ editor: e, range, props: mentionProps }) => {
            // 删除已有 mention
            const ranges: Array<[number, number]> = []
            e.state.doc.descendants((node: any, pos: number) => {
              if (node.type.name === 'mention') ranges.push([pos, pos + node.nodeSize])
            })
            const tr = e.state.tr
            for (let i = ranges.length - 1; i >= 0; i--) {
              tr.delete(ranges[i][0], ranges[i][1])
            }

            // 删除 @触发字符 + 查询文本
            // range 需要映射到删除 mention 后的新位置
            let adjustedFrom = range.from
            let adjustedTo = range.to
            for (const [delFrom, delTo] of ranges) {
              if (delTo <= range.from) {
                const delta = delTo - delFrom
                adjustedFrom -= delta
                adjustedTo -= delta
              }
            }
            tr.delete(adjustedFrom, adjustedTo)

            // 插入新 mention
            const mentionNode = e.state.schema.nodes.mention.create({
              id: mentionProps.id,
              label: mentionProps.label,
              mentionSuggestionChar: '@',
            })
            tr.insert(adjustedFrom, mentionNode)
            // 光标移到 mention 后
            const cursorPos = adjustedFrom + mentionNode.nodeSize
            tr.setSelection(TextSelection.create(tr.doc, cursorPos))
            tr.setMeta('mentionReplace', true)
            e.view.dispatch(tr)
          },
          items: ({ query }: { query: string }) => {
            return instances.value
              .filter(inst => {
                const searchStr = `${inst.name} ${inst.platform_type}`.toLowerCase()
                return searchStr.includes(query.toLowerCase())
              })
              .map(inst => ({
                id: inst.id,
                name: inst.name,
                label: getInstanceDisplayName(platformTypes.value, inst),
                typeName: platformTypes.value.find(t => t.key === inst.platform_type)?.name || inst.platform_type,
                platform_type: inst.platform_type,
              }))
          },
          render: () => {
            let component: any
            let popup: any

            return {
              onStart: (props: any) => {
                component = new VueRenderer(MentionList, {
                  props,
                  editor: props.editor,
                })
                if (!props.clientRect) return

                const windowH = window.innerHeight
                const rect = props.clientRect()
                const bottomSpace = windowH - rect.bottom - 44
                const topSpace = rect.top - 36

                const isFlipped = bottomSpace < 80 && topSpace > bottomSpace
                const availablePx = isFlipped ? topSpace : bottomSpace

                const finalH = Math.max(90, availablePx)
                document.documentElement.style.setProperty('--mention-list-max-h', `${finalH}px`)

                popup = tippy('body', {
                  getReferenceClientRect: props.clientRect,
                  appendTo: () => document.body,
                  content: component.element,
                  showOnCreate: true,
                  interactive: true,
                  trigger: 'manual',
                  placement: isFlipped ? 'top-start' : 'bottom-start',
                  theme: 'mention',
                  arrow: false,
                  offset: [0, 4],
                })
              },
              onUpdate(props: any) {
                component?.updateProps(props)
                popup?.setProps?.({
                  getReferenceClientRect: props.clientRect,
                })
              },
              onKeyDown(props: any) {
                if (props.event.key === 'Escape') {
                  popup?.hide?.()
                  return true
                }
                return component?.ref?.onKeyDown(props.event) ?? false
              },
              onExit() {
                document.documentElement.style.removeProperty('--mention-list-max-h')
                popup?.destroy?.()
                component?.destroy()
              },
            }
          },
        },
      }),
    ],
    editorProps: { attributes: { class: 'editor-content' } },
    onUpdate: ({ editor: e }) => {
      saveStatus.value = 'unsaved'
      autoSave()
      updateWordCount(e)
      // 通知外部 mention 变化
      if (onMentionChange) {
        let mentionId: string | null = null
        e.state.doc.descendants((node: any) => {
          if (node.type.name === 'mention' && node.attrs.id) {
            mentionId = node.attrs.id
          }
        })
        onMentionChange(mentionId)
      }
    },
  })

  // ── 字数统计 ──
  function updateWordCount(e: any) {
    if (!e) return
    const text = e.getText()
    const chinese = text.match(/[\u4e00-\u9fa5]/g) || []
    const english = text.match(/[a-zA-Z]+/g) || []
    wordCount.value = chinese.length + english.length
    charCount.value = text.length
  }

  // ── 自动保存（防抖 800ms）──
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  function autoSave() {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => doSave(), 800)
  }

  async function doSave() {
    if (!editor.value) return
    saveStatus.value = 'saving'
    try {
      const content = JSON.stringify(editor.value.getJSON())
      await invoke('save_note', { content })
      saveStatus.value = 'saved'
      setTimeout(() => { if (saveStatus.value === 'saved') saveStatus.value = 'idle' }, 2000)
    } catch (e) {
      console.error('保存失败:', e)
      saveStatus.value = 'unsaved'
    }
  }

  // ── 退出前强制保存 ──
  let unlistenExit: any = null

  async function handleExitRequest() {
    if (saveTimer) { clearTimeout(saveTimer); saveTimer = null }
    await doSave()
    await invoke('request_quit')
  }

  // ── 生命周期 ──
  onMounted(async () => {
    try {
      const content = await invoke<string>('read_note')
      if (content && editor.value) {
        try {
          const parsed = JSON.parse(content)
          editor.value.commands.setContent(parsed)
        } catch {
          editor.value.commands.setContent(content)
        }
        updateWordCount(editor.value)
      }
    } catch (e) {
      console.error('加载笔记内容失败:', e)
    }

    unlistenExit = await getCurrentWindow().listen('app-exit-request', handleExitRequest)
  })

  onBeforeUnmount(() => {
    if (saveTimer) clearTimeout(saveTimer)
    unlistenExit?.()
    editor.value?.destroy()
  })

  return {
    editor,
    saveStatus,
    wordCount,
    charCount,
    getMentionId,
    setMention,
    setOnMentionChange,
  }
}