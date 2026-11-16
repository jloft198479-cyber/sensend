// ═══ 斜杠命令扩展 ═══
// 行首输入 "/" 弹出命令列表；命令项全部集中在 COMMANDS 注册表，每项可用
// when 限定上下文——列表随光标位置自动增减，上下文不匹配的命令不出现。
// 与 @mention 同构：Suggestion 插件 + VueRenderer 弹层 + tippy 定位。
//
// 可插拔边界：
//   · 接入 = useEditor.ts 扩展数组里挂 SlashCommandExtension（一行）
//   · 下线 = 拔掉那一行，无其他引用点
//   · 扩展 = 往 COMMANDS 添一项；when 不填 = 任何上下文可用
import { Extension } from '@tiptap/core'
import { Suggestion } from '@tiptap/suggestion'
import { VueRenderer } from '@tiptap/vue-3'
import tippy from 'tippy.js'
import SlashList from '../components/SlashList.vue'

/** 斜杠命令项：run 收 editor，命令自己负责链式调用；when 缺省 = 任何上下文可用 */
export interface SlashCommandItem {
  id: string
  label: string
  hint: string
  run: (editor: any) => void
  /** 上下文限位：返回 false 则不进入当前光标下的命令列表（如表格命令只在表格内出现） */
  when?: (editor: any) => boolean
}

const COMMANDS: SlashCommandItem[] = [
  {
    id: 'table',
    label: '插入表格',
    hint: '3×3 带表头',
    // 已在表格内不出现，防止套娃插表
    when: editor => !editor.isActive('table'),
    run: editor => editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(),
  },
  {
    id: 'table-add-row',
    label: '表格加行',
    hint: '在当前行下方插入',
    when: editor => editor.isActive('table'),
    run: editor => editor.chain().focus().addRowAfter().run(),
  },
  {
    id: 'table-delete-row',
    label: '表格删行',
    hint: '删除光标所在行',
    when: editor => editor.isActive('table'),
    run: editor => editor.chain().focus().deleteRow().run(),
  },
  {
    id: 'table-add-col',
    label: '表格加列',
    hint: '在当前列右侧插入',
    when: editor => editor.isActive('table'),
    run: editor => editor.chain().focus().addColumnAfter().run(),
  },
  {
    id: 'table-delete-col',
    label: '表格删列',
    hint: '删除光标所在列',
    when: editor => editor.isActive('table'),
    run: editor => editor.chain().focus().deleteColumn().run(),
  },
  {
    id: 'table-delete',
    label: '删除整个表格',
    hint: '误删可 Ctrl+Z 撤销',
    when: editor => editor.isActive('table'),
    run: editor => editor.chain().focus().deleteTable().run(),
  },
  {
    id: 'blockquote',
    label: '引用',
    hint: '本行转为引用块',
    run: editor => editor.chain().focus().toggleBlockquote().run(),
  },
  {
    id: 'bullet-list',
    label: '无序列表',
    hint: '圆点列表',
    run: editor => editor.chain().focus().toggleBulletList().run(),
  },
  {
    id: 'ordered-list',
    label: '有序列表',
    hint: '数字列表',
    run: editor => editor.chain().focus().toggleOrderedList().run(),
  },
  {
    id: 'task-list',
    label: '待办',
    hint: '可勾选清单',
    run: editor => editor.chain().focus().toggleTaskList().run(),
  },
]

export const SlashCommandExtension = Extension.create({
  name: 'slashCommand',

  addProseMirrorPlugins() {
    return [
      Suggestion<SlashCommandItem>({
        editor: this.editor,
        char: '/',
        startOfLine: true,
        // 代码块里不弹：写注释、路径、网址时 "/" 很常见
        allow: ({ state, range }) => state.doc.resolve(range.from).parent.type.name !== 'codeBlock',
        items: ({ editor, query }) => {
          const q = query.trim().toLowerCase()
          return COMMANDS.filter(c =>
            (!c.when || c.when(editor)) && `${c.label} ${c.id}`.toLowerCase().includes(q),
          )
        },
        command: ({ editor, range, props }) => {
          // 先删掉 "/查询词"，再执行命令
          editor.chain().focus().deleteRange(range).run()
          props.run(editor)
        },
        render: () => {
          let component: any
          let popup: any

          return {
            onStart: (props: any) => {
              component = new VueRenderer(SlashList, { props, editor: props.editor })
              if (!props.clientRect) return

              const rect = props.clientRect()
              // 单行弹层，不做 @ 列表那套逐行算高：贴底就往上翻
              const isFlipped = rect.bottom + 120 > window.innerHeight && rect.top > 120
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
              popup?.setProps?.({ getReferenceClientRect: props.clientRect })
            },
            onKeyDown(props: any) {
              if (props.event.key === 'Escape') {
                popup?.hide?.()
                return true
              }
              return component?.ref?.onKeyDown(props.event) ?? false
            },
            onExit() {
              popup?.destroy?.()
              component?.destroy?.()
            },
          }
        },
      }),
    ]
  },
})
