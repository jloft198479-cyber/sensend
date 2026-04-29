import type { Ref } from 'vue'

/**
 * 编辑器格式操作 composable
 * 只负责 BubbleMenu 相关的格式切换
 */
export function useEditorFormat(editor: Ref<any>) {
  function toggleBold() { editor.value?.chain().focus().toggleBold().run() }
  function toggleItalic() { editor.value?.chain().focus().toggleItalic().run() }
  function toggleStrike() { editor.value?.chain().focus().toggleStrike().run() }
  function toggleH1() { editor.value?.chain().focus().toggleHeading({ level: 1 }).run() }
  function toggleH2() { editor.value?.chain().focus().toggleHeading({ level: 2 }).run() }
  function toggleBulletList() { editor.value?.chain().focus().toggleBulletList().run() }
  function toggleOrderedList() { editor.value?.chain().focus().toggleOrderedList().run() }
  function toggleBlockquote() { editor.value?.chain().focus().toggleBlockquote().run() }
  function toggleCode() { editor.value?.chain().focus().toggleCode().run() }

  function isActive(name: string, attrs?: Record<string, unknown>) {
    return editor.value?.isActive(name, attrs) ?? false
  }

  return {
    toggleBold,
    toggleItalic,
    toggleStrike,
    toggleH1,
    toggleH2,
    toggleBulletList,
    toggleOrderedList,
    toggleBlockquote,
    toggleCode,
    isActive,
  }
}