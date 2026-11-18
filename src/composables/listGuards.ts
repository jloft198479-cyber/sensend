// ═══ 列表结构守护（自 solo 同款守卫移植，2026-11-18）═══
// 病理（solo list-backspace-guard §2.1 实测复现，sensend 同样中招）：
//   「- 甲 / - 乙 / - 丙」中间项行首按退格，上游 ListKeymap 走 liftListItem →
//   liftOutOfList，塞空壳且不做 join ⇒ 列表被切成两个独立容器、中间夹一个
//   doc 层孤立段落，原样落盘、重开结构不变——这正是用户报的「列表脱节/断开」
//   与「发送出去全是碎列表」的真因（2026-11-18 排查确认）。
//   父项带子列表时行尾回车走 splitListItem，会把整棵子列表吃进新项（solo
//   2026-10-19 同款报告）。
// 药：两个 priority 101 守卫压过上游 keymap（默认 100），按契约表接管；
//   判定不成立一律 return false 放行上游。边界取舍见各 handler 注释。
// 出站安全：守卫只改「按键时结构如何变化」，产出的节点类型与 schema 完全一致，
//   ir.rs → 平台渲染管道不受影响（golden 测试兜底）。
import { Extension, type Editor } from '@tiptap/core'
import { Fragment } from '@tiptap/pm/model'
import type { Node as PMNode, ResolvedPos } from '@tiptap/pm/model'
import { TextSelection } from '@tiptap/pm/state'
import type { Transaction } from '@tiptap/pm/state'

// ── 列表类型常量（单一真源，solo list-move 同款）──
const LIST_NODE_TYPES = new Set(['bulletList', 'orderedList', 'taskList'])
const ITEM_NODE_TYPES = new Set(['listItem', 'taskItem'])

/** 输入法组字中判定：组字期绝不做结构事务（结构重排会打断 IME 候选窗锚定）。 */
function isFrozen(view: Editor['view']): boolean {
  return view?.composing === true
}

/** 从光标位置向上找最近的项容器深度；不在任何项内返回 -1 */
function itemDepthAt($pos: ResolvedPos): number {
  for (let d = $pos.depth; d > 0; d--) {
    if (ITEM_NODE_TYPES.has($pos.node(d).type.name)) return d
  }
  return -1
}

/** 光标所在项是否嵌套（项容器之上还有另一层项容器） */
function isNestedItem($from: ResolvedPos, itemDepth: number): boolean {
  for (let d = itemDepth - 2; d > 0; d--) {
    if (ITEM_NODE_TYPES.has($from.node(d).type.name)) return true
  }
  return false
}

// ── 纯空壳列表项（solo list-shell 同款）──
// 空壳：项内没有任何文字，孩子全是（或只剩）子列表。markdown 落成 `- - 甲`
// 这种结构，屏幕上留下没有内容的圆点壳。sensend listItem content =
// 'paragraph block+'，只装子列表的项在 schema 里合法，机器校验拦不住，
// 只能由结构出口自己保证不产生它。

/** 纯空壳判定：空段落孩子（无文字）不打破 onlyLists */
function isPureShell(node: PMNode): boolean {
  if (!ITEM_NODE_TYPES.has(node.type.name)) return false
  let onlyLists = true
  node.forEach((c) => {
    if (!LIST_NODE_TYPES.has(c.type.name) && c.textContent.trim().length > 0) onlyLists = false
  })
  return onlyLists
}

/**
 * 溶解纯空壳：子列表项提升到父列表（同类型展开成兄弟项；类型不同但父容器
 * 收得下就整块搬过去）。保守原则：任何子列表塞不进父容器就整体放弃、不改文档
 * ——宁可留壳，也不写出会被 PM 补成畸形甚至抛错的结构。调用方据 tr.steps.length
 * === 0 判断是否真的改动过。
 */
function dissolveShell(tr: Transaction, shellPos: number): void {
  const shell = tr.doc.nodeAt(shellPos)
  if (!shell || !isPureShell(shell)) return
  const $pos = tr.doc.resolve(shellPos + 1)
  const itemDepth = itemDepthAt($pos)
  if (itemDepth < 1) return
  const parentList = $pos.node(itemDepth - 1)
  if (!LIST_NODE_TYPES.has(parentList.type.name)) return
  const promoted: PMNode[] = []
  let compatible = true
  shell.forEach((c) => {
    if (!LIST_NODE_TYPES.has(c.type.name)) return
    if (c.type.name === parentList.type.name) c.forEach((gc) => promoted.push(gc))
    else if (parentList.type.contentMatch.matchType(c.type)) promoted.push(c)
    else compatible = false
  })
  if (!compatible) return
  if (promoted.length === 0) tr.delete(shellPos, shellPos + shell.nodeSize)
  else tr.replaceWith(shellPos, shellPos + shell.nodeSize, Fragment.from(promoted))
}

// ═══ 行首退格守卫：永不劈开列表 ═══
// 契约表（solo §七 九条决定 + 复审修订，sensend 场景逐条适用）：
// | 光标所在（顶层多项列表） | 行首 Backspace 行为                          |
// |---|---|
// | 非首项 | 与上一项合并（并成一段；两次 joinBackward 防隐藏第二段） |
// | 首项，前面是顶层块 | lift 出来再并进前一块；join 不成则只 lift（不劈） |
// | 首项，列表是文档第一块 | 不动（够不着就不动） |
// | 首块是空 textblock | 纯壳整壳溶解（子项升一级）；有普通块则只删空块； |
// |                      | 单项空列表放行上游（= 退出列表 / 撤输入规则出口） |
// | 单项列表 | 放行上游 lift（退出列表，保持现状） |
// | 嵌套子项 | 首项放行上游 lift（整体提升一级）；非首项并进上一项 |

interface ItemContext {
  item: PMNode
  itemDepth: number
  /** 项在所属列表容器里的序号 */
  index: number
}

/** 判定光标是否处于「项的第一个子块的开头」，是则返回项上下文 */
function resolveGuardContext(editor: Editor): ItemContext | null {
  const { selection } = editor.state
  if (!(selection instanceof TextSelection) || !selection.empty) return null
  const { $from } = selection
  if ($from.parentOffset !== 0) return null
  const itemDepth = itemDepthAt($from)
  if (itemDepth < 0) return null
  const item = $from.node(itemDepth)
  // 只接管「第一个子块」的开头；项内第二段开头的默认段落合并本来安全
  if ($from.depth !== itemDepth + 1 || item.firstChild !== $from.parent) return null
  // 项在列表容器里的序号：从容器首项逐项累加 nodeSize
  const itemStart = $from.before(itemDepth)
  let cursor = $from.before(itemDepth - 1) + 1
  let index = 0
  const list = $from.node(itemDepth - 1)
  while (cursor < itemStart) {
    cursor += list.child(index).nodeSize
    index++
  }
  return { item, itemDepth, index }
}

function handleListBackspace(editor: Editor): boolean {
  const context = resolveGuardContext(editor)
  if (!context) return false
  const { view } = editor
  // 组字门控：只在「本守卫即将接管结构变更」的键位上吞掉；不满足接管条件的
  // 键照常穿透（保持上游既有行为）
  if (isFrozen(view)) return true
  const { item, itemDepth, index } = context
  const { state } = editor
  const list = state.selection.$from.node(itemDepth - 1)
  const firstBlock = item.firstChild as PMNode

  // 首块是空 textblock：
  // ① 空段 + 其余全是子列表（纯空壳）⇒ 整壳溶解、子项升一级。只删空段会留下
  //    没有正文的圆点壳（li::marker 与子项 marker 挤同一行，落盘 `- - 甲`）。
  // ② 项里还有普通块 ⇒ 只删这个空块，项结构不动（产物合法）。
  // ③ 空段是唯一子块 ⇒ 绝不能只删块——会留下零子块 listItem（违反 schema
  //    'paragraph block+'），圆点壳残留且上游命令全部失效 =「列表删不掉」。
  //    删掉整个空项；单项空列表仍放行上游——那是「退出列表」+「撤销 - 输入
  //    规则」的出口（undoInputRule 在上游 handleBackspace 第一行），必须保持。
  if (firstBlock.isTextblock && firstBlock.content.size === 0) {
    const tr = state.tr
    // childCount > 1 前置不能省：单项空列表（只有空段、没有子列表）也满足
    // isPureShell，若走溶解会把它整项删掉——那条路必须留给「退出列表」的上游 lift
    if (item.childCount > 1 && isPureShell(item)) {
      const itemStart = state.selection.$from.before(itemDepth)
      dissolveShell(tr, itemStart)
      if (tr.steps.length > 0) {
        // 光标跟到前一项末尾（壳是首项时落到提升上来的首项开头）
        tr.setSelection(
          index > 0
            ? TextSelection.near(tr.doc.resolve(itemStart - 1), -1)
            : TextSelection.near(tr.doc.resolve(itemStart), 1),
        )
        view.dispatch(tr)
        return true
      }
      // 溶解不成立（子列表塞不进父容器）⇒ 退一步仍按「只删空块」处理
    }
    if (item.childCount > 1) {
      const from = state.selection.$from.before(itemDepth + 1)
      const to = state.selection.$from.after(itemDepth + 1)
      tr.delete(from, to)
      tr.setSelection(TextSelection.near(tr.doc.resolve(from), -1))
      view.dispatch(tr)
      return true
    }
    if (list.childCount === 1) return false
    const from = state.selection.$from.before(itemDepth)
    const to = state.selection.$from.after(itemDepth)
    tr.delete(from, to)
    tr.setSelection(TextSelection.near(tr.doc.resolve(from), -1))
    view.dispatch(tr)
    return true
  }

  // 单项列表：行首退格 = 退出列表（上游 lift 行为正确，保持现状）
  if (list.childCount === 1) return false

  // 嵌套项的首项：放行上游 lift——整体提升一级、圆点与子树都在；走下面的
  // lift+joinBackward 会把「列表与列表」合并降级成父项里的第二段。嵌套非首项
  // 不受影响，仍并进上一项
  if (index === 0 && isNestedItem(state.selection.$from, itemDepth)) return false

  if (index > 0) {
    // 非首项：并进上一项（同容器兄弟），永不劈开。一次 joinBackward 会留项内
    // 第二段（肉眼难辨的隐藏段落），连跑两次才并成一段；第二步不成（末块类型
    // 不同等）退一步只做项级合并——留两块好过劈成两段
    if (editor.chain().joinBackward().joinBackward().run()) return true
    return editor.chain().joinBackward().run()
  }

  // 首项 + 多项：列表是文档第一块 ⇒ 不动（够不着就不动）
  const listStart = state.selection.$from.before(itemDepth - 1)
  if (listStart === 0) return true

  // 前面有顶层块：把首项 lift 出来并进前一块；join 不成（前块是 heading 等
  // 类型不兼容）则只 lift——首项 lift 实测不劈
  const name = item.type.name
  if (editor.chain().liftListItem(name).joinBackward().run()) return true
  return editor.chain().liftListItem(name).run()
}

/**
 * 顶层项内 Shift-Tab：一律吞掉、不改文档（顶层项「−1」越界 ⇒ no-op）。
 * 上游 Shift-Tab 走 liftListItem → liftOutOfList 会拆掉容器（劈列同族），
 * 顶层项缩进已是 0、没有合法动作可做 ⇒ 吞掉保结构。嵌套子项放行上游正常反缩进。
 * 注意门槛只看「在不在项内」，不能要求光标停在项首块 offset 0——Shift-Tab 是
 * 随手按的，光标多半停在字中间，旧门槛覆盖不到的位置会落到上游拆容器。
 */
function handleListShiftTab(editor: Editor): boolean {
  const { selection } = editor.state
  if (!(selection instanceof TextSelection) || !selection.empty) return false
  const { $from } = selection
  const itemDepth = itemDepthAt($from)
  if (itemDepth < 0) return false // 不在任何列表项内：放行
  if (isFrozen(editor.view)) return true
  if (isNestedItem($from, itemDepth)) return false // 嵌套子项放行上游（正常反缩进）
  return true
}

export const ListBackspaceGuard = Extension.create({
  name: 'listBackspaceGuard',
  // 必须 ≥101 才压得过上游 ListItem/ListKeymap 的默认 100（tiptap 每扩展各成
  // 一个 keymap 插件，按 priority 降序询问）
  priority: 101,
  addKeyboardShortcuts() {
    return {
      Backspace: () => handleListBackspace(this.editor),
      // Mod-Backspace 与 Backspace 同键同治；判定不成立时返回 false 放行，
      // 特殊块（代码块等）的 Mod-Backspace 整块删除语义不受影响
      'Mod-Backspace': () => handleListBackspace(this.editor),
      'Shift-Tab': () => handleListShiftTab(this.editor),
    }
  },
})
// 病：`- A` 下挂 `- A1`，光标在 A 末尾按 Enter，上游 splitListItem 在段落末尾
// 切整项——切点之后的一切（包括子列表）都归新项 ⇒ `- A / - (空) /   - A1`，
// 新项凭空多了一群孩子，A 反而失去了 A1。
// 药：只在「项内首块末尾 + 首块之后全是子列表」时接管，在整项之后（含子列表）
// 插入同类型空项——子列表留在原项名下，新项是干净的同级空项。
// 边界（有意不接管，原样走上游）：
// - 光标不在项内首块（项内第三段、子项里）⇒ 上游 split 本来就是对的
// - 光标不在首块末尾（行中回车）⇒ 行中切分归属不变
// - 首块之后混有普通块（项内续行/多段落项）⇒ 那是在项内续行，不是大纲换项
// - 单块项（没有子列表）⇒ 上游行为已正确
// 例外（接管）：列表项内的代码块——上游 keymap 顺序让 ListItem 抢走回车，
// 按 newlineInCode 语义补一个换行，保证列表项里代码块也能正常换行。

/** 首块之后是否全是子列表（＝一条标准大纲项；混入普通块则是在项内续行） */
function restAreChildLists(item: PMNode): boolean {
  if (item.childCount < 2) return false
  for (let i = 1; i < item.childCount; i++) {
    if (!LIST_NODE_TYPES.has(item.child(i).type.name)) return false
  }
  return true
}

function handleListEnter(editor: Editor): boolean {
  const { state } = editor
  const { selection } = state
  if (!(selection instanceof TextSelection) || !selection.empty) return false
  const { $from } = selection

  // 列表项内的代码块：Enter = 在代码里换行（与文档级代码块一致）。
  // 上游顺序坑：tiptap 按反转后的扩展表收集 keymap，同 priority 时 ListItem 的
  // Enter 排在 core Keymap 的 newlineInCode 之前 ⇒ 列表项里代码块的回车会拆项，
  // 子列表还会被新项带走。这里按 newlineInCode 的语义补回换行
  if ($from.parent.type.spec.code && itemDepthAt($from) > 0) {
    if (isFrozen(editor.view)) return true
    editor.view.dispatch(state.tr.insertText('\n'))
    return true
  }

  const itemDepth = itemDepthAt($from)
  if (itemDepth < 0) return false
  const item = $from.node(itemDepth)
  // 只认「项内首块 + 光标在该块末尾」：项内续行的回车保持上游行为
  if ($from.depth !== itemDepth + 1 || item.firstChild !== $from.parent) return false
  if ($from.parentOffset !== $from.parent.content.size) return false
  if (!restAreChildLists(item)) return false
  // 组字门控：组字期绝不做结构事务
  if (isFrozen(editor.view)) return true

  const newItem = item.type.createAndFill()
  if (!newItem) return false

  // 整项之后：子列表仍在原项名下，新项是同级的干净空项
  const insertPos = $from.after(itemDepth)
  const tr = state.tr.insert(insertPos, newItem)
  tr.setSelection(TextSelection.near(tr.doc.resolve(insertPos + 1), 1))
  editor.view.dispatch(tr.scrollIntoView())
  return true
}

export const ListEnterGuard = Extension.create({
  name: 'listEnterGuard',
  priority: 101,
  addKeyboardShortcuts() {
    return {
      Enter: () => handleListEnter(this.editor),
    }
  },
})
