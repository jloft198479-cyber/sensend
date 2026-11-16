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
import { useToast } from './useToast'
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
import { getInstanceDisplayName, toInstanceItem } from '../types/platform'
import MentionList from '../components/MentionList.vue'
import { SentMarkExtension, markSent } from './useSentMark'

// [ZC-06] 零实例引导的会话级去重：一个会话只提示一次，不让每次按 @ 都弹 toast
let zeroInstanceTipShown = false

/**
 * 编辑器核心 composable
 * 职责：TipTap 初始化、自动保存、字数统计、@mention
 * 格式操作 → useEditorFormat
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

  // ── [OPT] 差异检测状态 ──
  let lastMentionId: string | null = null          // mention 缓存
  let wordCountTimer: ReturnType<typeof setTimeout> | null = null  // 字数防抖
  let lastSavedContent: string | null = null       // 保存差异检测

  // ── [C-01 修复] 笔记读取成功前禁止保存 ──
  // 读取失败/还没读完时，编辑器是空的、lastSavedContent 仍为 null，此时任何输入触发的
  // doSave 都会因差异检测恒成立而覆盖原笔记。用闸门拦住：默认关闭(noteLoadReady=false)，
  // 只有读到内容（含首用的空串）并把 lastSavedContent 初始化后才解锁；"加载中"和"读失败"
  // 两种状态一律不写盘——慢失败场景下也能堵住（若只在 catch 里拦，慢失败时会漏）。
  let noteLoadReady = false
  // 读取失败的可见状态：App.vue 据它渲染常驻提示条（含「重试」「打开数据目录」两个出口）。
  // 不用 toast —— toast 整条可点击关闭（ToastLayer.vue:16），点掉后既没有第二次触发入口，
  // 也会让用户误以为已经恢复，实际编辑器仍锁着，只能重启。
  const noteLoadFailed = ref(false)

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
      // 发送成功视觉标记（ProseMirror decoration，不碰文档 JSON）
      SentMarkExtension,
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
          // [ZC-06] 一个目标都没配时禁止弹出 @ 列表（allow 默认恒 true，会弹"无匹配平台"空壳），
          // 改为一次性引导用户去配置——allow=false 时按 @ 无反应，必须有这条 toast 才不像"按键坏了"
          allow: () => {
            if (instances.value.length > 0) return true
            if (!zeroInstanceTipShown) {
              zeroInstanceTipShown = true
              useToast().info('还没有配置发送目标，点底栏「管理」添加一个')
            }
            return false
          },
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
            // [ZC-05] 先映射成 InstanceItem、再筛选：匹配实例名 / 类型中文显示名（"飞书""本地文件夹"）/
            // 平台 key。此前只拼 name + platform_type（lark/local），用户照屏幕上的字打永远 0 条。
            // 映射规则仍然只有 toInstanceItem 一份（types/platform.ts 的 SSOT），此处不做第二份映射
            const q = query.toLowerCase()
            return instances.value
              .map(inst => toInstanceItem(inst, platformTypes.value))
              .filter(item =>
                `${item.name} ${item.typeName} ${item.platform_type}`.toLowerCase().includes(q),
              )
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
                // 几何值全部读取设计 token（单一来源 styles/base.css），此处不写字面量
                const rootStyle = getComputedStyle(document.documentElement)
                const token = (name: string, fallback: number) => {
                  const n = parseFloat(rootStyle.getPropertyValue(name))
                  return Number.isFinite(n) ? n : fallback
                }
                const edgeGap = token('--list-edge-gap', 12)
                const rowH = token('--menu-row-h', 28)
                const padY = token('--menu-pad-y', 6) * 2
                const maxRows = token('--mention-max-rows', 8)
                const flipMin = token('--mention-flip-min', 80)
                const footerH =
                  document.querySelector('.footer-bar')?.getBoundingClientRect().height ||
                  token('--footer-h', 46)

                const bottomSpace = windowH - rect.bottom - footerH - edgeGap
                const topSpace = rect.top - edgeGap

                const isFlipped = bottomSpace < flipMin && topSpace > bottomSpace
                const availablePx = isFlipped ? topSpace : bottomSpace

                // 高度按整行取整 + 上下内边距：避免末行被切一半，同时上下都有留白
                const rows = Math.max(1, Math.min(maxRows, Math.floor((availablePx - padY) / rowH)))
                const finalH = rows * rowH + padY
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
      // 已知成本（[m-28] 评审提过）：条件 3 成立期间，每个按键都会全量遍历一次文档（O(文档大小)）。
      // 这是有意的「正确性优先」取舍——改成「把 mention 位置按 tr.mapping 映射后只查那一个点」
      // 看起来更快，但 setContent / 整段替换时映射会给出失效位置，一旦误判成「mention 已删除」
      // 就会把底栏目标静默切走，代价远大于这点遍历。要优化必须先有长文档卡顿的实测证据。
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

  // 在途保存的 Promise：退出前必须等它完成，防止 app.exit 截断原子写丢数据
  let inFlightSave: Promise<void> | null = null

  async function doSave() {
    if (!editor.value) return
    // [C-01 修复] 读取笔记未成功确认前禁止保存（双保险）：加载中/读失败时编辑器已禁用，
    // 正常不会触发到这里；仍保留守卫拦截任何绕过路径（退出保存、快捷键、编程式改动），
    // 静默返回避免刷屏提示。
    if (!noteLoadReady) return
    // 防止并发保存（数据损坏级）：在途保存必须先串行等待落地。
    // 不能只看 saveStatus==='saving' 守卫——打字的 onUpdate 会把状态改回 'unsaved'，
    // 使下一次防抖到点绕过守卫；而 save_note 共用同一临时文件，并发写会交错损坏 note.json。
    if (inFlightSave) await inFlightSave.catch(() => {})

    // [OPT] 内容差异检测：序列化后与上次保存比较，相同则跳过磁盘 I/O
    const content = JSON.stringify(editor.value.getJSON())
    if (content === lastSavedContent) {
      saveStatus.value = 'idle'
      return
    }

    // 等待期间已有更晚触发的保存启动（多等待者排队），让位——最新内容由防抖链的后续保存兜底
    if (inFlightSave) return

    saveStatus.value = 'saving'
    inFlightSave = (async () => {
      try {
        await invoke('save_note', { content })
        lastSavedContent = content
        saveStatus.value = 'saved'
        setTimeout(() => { if (saveStatus.value === 'saved') saveStatus.value = 'idle' }, 2000)
      } catch (e) {
        console.error('保存失败:', e)
        saveStatus.value = 'unsaved'
      } finally {
        inFlightSave = null
      }
    })()
    await inFlightSave
  }

  // ── 退出前强制保存 ──
  let unlistenExit: any = null

  async function handleExitRequest() {
    if (saveTimer) { clearTimeout(saveTimer); saveTimer = null }
    if (wordCountTimer) { clearTimeout(wordCountTimer); wordCountTimer = null }
    // 无条件走 doSave：保存完成会无条件置 'saved'，可能顶掉在途期间打字置的 'unsaved'，
    // 若按状态判断会漏掉末次编辑（防抖定时器已被上面 clearTimeout 杀掉，再不存就永久丢）。
    // 差异检测使无变化时零成本跳过
    await doSave()
    // 等在途保存完成：saving 状态下退出，app.exit(0) 会截断原子写（tmp 未 rename），丢最后一次编辑
    if (inFlightSave) await inFlightSave
    await invoke('request_quit')
  }

  // ── 笔记加载（可重试）──
  // [C-01 修复] 加载一发起就锁编辑器+关保存闸门，只有读取成功才解锁。
  // 这样"加载中"和"读失败"两种状态都不写盘，慢失败场景也不会覆盖原笔记。
  let noteLoading = false   // 防重入：连点"重试"时只允许一个读取在途

  /** 读取笔记，带超时。invoke 若永不 settle，界面会卡在"编辑器锁死 + 没有任何提示"
   *  的状态里，重试也会被 noteLoading 吞掉——必须自己给一个上限。 */
  function readNoteWithTimeout(ms = 15_000): Promise<string> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('读取笔记超时')), ms)
      invoke<string>('read_note').then(
        (v) => { clearTimeout(timer); resolve(v) },
        (e) => { clearTimeout(timer); reject(e) },
      )
    })
  }

  async function loadNote(): Promise<void> {
    if (noteLoading) return
    noteLoading = true
    // 发起读取前先锁死：加载中的窗口内用户打字也不会触发保存/被 setContent 覆盖
    if (editor.value) editor.value.setEditable(false, false)  // 第二参 false：不 emit 伪 update
    try {
      const content = await readNoteWithTimeout()
      if (content && editor.value) {
        try {
          const parsed = JSON.parse(content)
          editor.value.commands.setContent(parsed, { emitUpdate: false })
        } catch {
          editor.value.commands.setContent(content, { emitUpdate: false })
        }
      }
      // 只有读取成功到这里才初始化基线并解锁保存（空串=首次使用，基线为空，属正常）
      if (editor.value) {
        lastSavedContent = JSON.stringify(editor.value.getJSON())
        lastMentionId = getMentionId()
        updateWordCount(editor.value)
        editor.value.setEditable(true, false)   // 读成功才恢复可编辑，且不 emit 伪 update
        // 闸门必须与"基线已初始化"同时成立：原来写在 if 外面，editor 尚未就绪时会
        // 解锁保存但 lastSavedContent 仍是 null → 差异检测恒成立 → 又回到覆盖原笔记
        noteLoadReady = true
        noteLoadFailed.value = false
      } else {
        // editor 没就绪：保持闸门关闭，别假装加载成功（提示条与闸门必须同进同出）
        noteLoadReady = false
        noteLoadFailed.value = true
      }
    } catch (e) {
      // 读取失败：保持闸门关闭 + 编辑器禁用（杜绝"白写"和覆盖），由 App.vue 的提示条给重试出口
      noteLoadReady = false
      noteLoadFailed.value = true
      if (editor.value) editor.value.setEditable(false, false)
      console.error('加载笔记内容失败:', e)
    } finally {
      noteLoading = false
    }
  }

  // ── 生命周期 ──
  onMounted(async () => {
    await loadNote()
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
    getMentionId,
    setMention,
    setOnMentionChange,
    markSent,
    // 读取失败的可见状态 + 重试入口（App.vue 常驻提示条使用）
    noteLoadFailed,
    retryLoadNote: () => { void loadNote() },
    // 供自动更新安装前善后调用（安装器会直接退出应用，不走 app-exit-request）
    saveNow: doSave,
    // 双保险 flush：等 doSave 落地并兜底等在途保存（与 handleExitRequest 同款模式），
    // 更新安装链的 beforeInstall 应使用本方法而非 saveNow
    flushSave: async () => {
      await doSave()
      if (inFlightSave) await inFlightSave
    },
  }
}
