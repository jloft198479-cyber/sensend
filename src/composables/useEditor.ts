// ═══ 优化方案 3：onUpdate 回调瘦身 ═══
// 改动说明（相对原版，用 [OPT] 标记所有改动点）：
// 1. [OPT] 新增 lastMentionId 缓存，onUpdate 中只在 mentionId 实际变化时才触发回调
//    —— 普通打字不改变 mention，避免了每次按键的 doc.descendants 遍历 + Vue 响应式更新
// 2. [OPT] 字数统计从 onUpdate 同步调用改为独立防抖（300ms）
//    —— 打字过程中不需要实时字数，300ms 延迟无感但减少了每次按键的正则开销
// 3. [OPT] doSave 增加内容差异检测（lastSavedContent），内容未变则跳过写入
//    —— 用户编辑后撤销回原样时，不再做无意义的磁盘 I/O
//
// 所有其他逻辑（mention 插入/删除、自动保存防抖、退出前保存）完全不变

import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useEditor as useTiptapEditor } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import Placeholder from '@tiptap/extension-placeholder'
import Mention from '@tiptap/extension-mention'
import TaskList from '@tiptap/extension-task-list'
import TaskItemBase from '@tiptap/extension-task-item'
import { TableKit } from '@tiptap/extension-table'
import { Markdown } from '@tiptap/markdown'
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
  /**
   * 网页 checkbox 兼容：网页复制的待办是 <li><input type=checkbox>文本</li>，
   * 原生 parseHTML 只认 li[data-type="taskItem"]，会丢 checkbox 退化成普通列表。
   * 这里补两条 :has 规则（priority 52 高于 listItem 的 50），并把 input 的勾选态读进 attrs。
   */
  const TaskItem = TaskItemBase.extend({
    parseHTML() {
      return [
        { tag: 'li[data-type="taskItem"]', priority: 51 },
        { tag: 'li:has(> input[type="checkbox"])', priority: 52 },
        { tag: 'li:has(> label input[type="checkbox"])', priority: 52 },
      ]
    },
    addAttributes() {
      const parent = (TaskItemBase.config.addAttributes as () => Record<string, any>)()
      return {
        ...parent,
        checked: {
          ...parent.checked,
          parseHTML: (el: HTMLElement) =>
            el.getAttribute('data-checked') === '' ||
            el.getAttribute('data-checked') === 'true' ||
            (el.querySelector('input[type="checkbox"]') as HTMLInputElement | null)?.checked === true,
        },
      }
    },
  })

  type SaveStatus = 'idle' | 'saved' | 'saving' | 'unsaved'
  const saveStatus = ref<SaveStatus>('idle')
  const wordCount = ref(0)
  const charCount = ref(0)

  // ── [OPT] 差异检测状态 ──
  let lastMentionId: string | null = null          // mention 缓存
  let wordCountTimer: ReturnType<typeof setTimeout> | null = null  // 字数防抖
  let lastSavedContent: string | null = null       // 保存差异检测

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
    // 打 mentionReplace 标记：onUpdate 快路径据此识别"这是 mention 变化"，必做全量检测
    editor.value.chain()
      .focus()
      .command(({ tr }) => {
        tr.setMeta('mentionReplace', true)
        return true
      })
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

  // ── [OPT] 字数统计（独立防抖 300ms）──
  // 从 onUpdate 同步调用改为防抖，减少每次按键的正则开销
  function scheduleWordCount(e: any) {
    if (wordCountTimer) clearTimeout(wordCountTimer)
    wordCountTimer = setTimeout(() => updateWordCount(e), 300)
  }

  function updateWordCount(e: any) {
    if (!e) return
    const text = e.getText()
    const chinese = text.match(/[\u4e00-\u9fa5]/g) || []
    const english = text.match(/[a-zA-Z]+/g) || []
    wordCount.value = chinese.length + english.length
    charCount.value = text.length
  }

  // ── 编辑器 ──
  const editor = useTiptapEditor({
    content: '',
    extensions: [
      StarterKit,
      // 待办列表：taskList + taskItem，勾选状态存 attrs.checked（四个适配器均已支持）
      TaskList,
      TaskItem.configure({ nested: true }),
      // 表格支持：从网页/其他编辑器复制含表格的 HTML 时能正确解析
      TableKit,
      // Markdown 支持：提供 parse/serialize 能力，配合 handlePaste 处理纯文本 Markdown 粘贴
      Markdown,
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
    editorProps: {
      attributes: { class: 'editor-content' },
      handlePaste: (_view, event: ClipboardEvent) => {
        const html = event.clipboardData?.getData('text/html')
        // 含 checkbox 的 HTML（网页复制的待办）：先改造成 TipTap 标准结构再插入。
        // 不处理的话：checkbox 被 parseHTML 丢弃 + 留下空 bulletList 壳（导出会多一个空列表项）
        if (html && /<input[^>]+type=["']?checkbox/i.test(html)) {
          const doc = new DOMParser().parseFromString(html, 'text/html')
          doc.querySelectorAll('ul').forEach(ul => {
            const lis = Array.from(ul.children).filter(el => el.tagName === 'LI')
            if (!lis.length || !lis.some(li => li.querySelector('input[type="checkbox"]'))) return
            ul.setAttribute('data-type', 'taskList')
            lis.forEach(li => {
              const cb = li.querySelector('input[type="checkbox"]') as HTMLInputElement | null
              if (!cb) return
              li.setAttribute('data-type', 'taskItem')
              li.setAttribute('data-checked', String(cb.checked))
              // 拆掉 label 包裹、删掉 checkbox 本体，只留纯文本/子结构
              li.querySelectorAll('label').forEach(l => l.replaceWith(...Array.from(l.childNodes)))
              li.querySelectorAll('input[type="checkbox"]').forEach(n => n.remove())
            })
          })
          editor.value?.commands.insertContent(doc.body.innerHTML)
          return true
        }
        // 其他 HTML：让默认处理（TableKit 已注册，能解析表格）
        if (html && html.trim()) return false

        // 纯文本：检测 Markdown 标记，用 Markdown 扩展解析
        const text = event.clipboardData?.getData('text/plain') || ''
        if (!text) return false

        const manager = (editor.value?.storage as any)?.markdown?.manager
        if (!manager) return false

        // 粗判是否含 Markdown 语法（标题/列表/引用/代码块/分隔线/粗体/行内代码/链接/表格分隔行）
        // 表格分支：形如 |---|---| 的分隔行（行内只含 |:-空格 且有连续两个以上 -），
        // 普通句子里的单竖线不会命中
        const looksLikeMarkdown = /(^|\n)(#{1,6}\s|[-*+]\s|\d+\.\s|>\s|```|---|\s*\|?[\s:|-]*-{2,}[\s:|-]*\|)|\*\*[^*]+\*\*|__[^_]+__|`[^`]+`|\[[^\]]+\]\([^)]+\)/.test(text)
        if (!looksLikeMarkdown) return false

        try {
          const json = manager.parse(text)
          if (!json?.content || json.content.length === 0) return false
          // 解析后仍是单段纯文本 → 当普通文本处理
          if (json.content.length === 1 && json.content[0].type === 'paragraph') {
            const inner = json.content[0].content
            if (!inner || inner.length <= 1) return false
          }
          editor.value?.commands.insertContent(json)
          return true
        } catch {
          return false
        }
      },
    },
    onUpdate: ({ editor: e, transaction: tr }) => {
      saveStatus.value = 'unsaved'
      autoSave()

      // [OPT] 字数统计：防抖 300ms，不再每次按键同步执行正则
      scheduleWordCount(e)

      // [OPT] mention 快路径：三种情况才值得全量遍历，其余（普通打字）直接跳过
      // 1. 本事务是 mention 替换（setMention / suggestion.command 打的标记）
      // 2. 删过字符（退格/选中删除可能删掉 mention，from !== to）
      // 3. 文中已有 mention（lastMentionId 非空时任何变化都可能影响它）
      // 注：不能用 stepType !== 'replace' 判断——插入也是 ReplaceStep，恰好会漏掉唯一需检测的场景
      if (onMentionChange) {
        const isMentionReplace = tr.getMeta('mentionReplace') === true
        const deletedSomething = tr.steps.some(s => (s as any).from !== (s as any).to)
        if (isMentionReplace || deletedSomething || lastMentionId !== null) {
          let currentMentionId: string | null = null
          e.state.doc.descendants((node: any) => {
            if (node.type.name === 'mention' && node.attrs.id) {
              currentMentionId = node.attrs.id
            }
          })
          if (currentMentionId !== lastMentionId) {
            lastMentionId = currentMentionId
            onMentionChange(currentMentionId)
          }
        }
      }
    },
  })

  // ── 自动保存（防抖 800ms）──
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  function autoSave() {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => doSave(), 800)
  }

  async function doSave() {
    if (!editor.value) return

    // [OPT] 内容差异检测：序列化后与上次保存比较，相同则跳过磁盘 I/O
    const content = JSON.stringify(editor.value.getJSON())
    if (content === lastSavedContent) {
      saveStatus.value = 'idle'
      return
    }

    saveStatus.value = 'saving'
    try {
      await invoke('save_note', { content })
      lastSavedContent = content
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
    if (wordCountTimer) { clearTimeout(wordCountTimer); wordCountTimer = null }
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
        // [OPT] 初始化 lastSavedContent，避免首次 setContent 后立即触发保存
        lastSavedContent = JSON.stringify(editor.value.getJSON())
        // [OPT] 同步 lastMentionId 与已加载文档（setContent 不触发 onUpdate，缓存需手动对齐）
        lastMentionId = getMentionId()
        updateWordCount(editor.value)
      }
    } catch (e) {
      console.error('加载笔记内容失败:', e)
    }

    unlistenExit = await getCurrentWindow().listen('app-exit-request', handleExitRequest)
  })

  onBeforeUnmount(() => {
    if (saveTimer) clearTimeout(saveTimer)
    if (wordCountTimer) clearTimeout(wordCountTimer)
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
