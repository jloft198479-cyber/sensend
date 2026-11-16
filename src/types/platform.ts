export interface PlatformInstance {
  id: string
  name: string
  platform_type: string
  token: string
  token2?: string  // 第二个凭证字段（飞书 App Secret）
  target_id: string
  publish_mode: string  // "page" | "block"
}

export interface PlatformTypeInfo {
  key: string
  name: string
  fields: PlatformField[]
  /** 平台支持的写入模式（SSOT：后端下发；决定「写入方式」显隐与合法性，缺省视为 page/block 均可） */
  supported_modes?: string[]
}

export interface PlatformField {
  key: string
  label: string
  hint: string
  secret: boolean
  hidden?: boolean
  browse?: boolean
  default_value?: string
  optional?: boolean
}

export interface PublishResult {
  success: boolean
  message: string
  url: string | null
}

/** 统一实例显示名称：实例名-平台类型（如“随心记-flowus”） */
export function getInstanceDisplayName(types: PlatformTypeInfo[], inst: PlatformInstance): string {
  const typeName = types.find(t => t.key === inst.platform_type)?.name || inst.platform_type
  return `${inst.name}-${typeName}`
}

/** 实例选择列表的统一条目格式（底栏目标弹窗 / 编辑器 @ 列表共用，真理源自此处） */
export interface InstanceItem {
  id: string
  name: string
  label: string        // 完整显示名（实例名-类型），供 mention 节点与 aria 使用
  typeName: string
  platform_type: string
}

/** 平台实例 → 统一条目（唯一的映射规则，勿在他处重拼） */
export function toInstanceItem(inst: PlatformInstance, types: PlatformTypeInfo[]): InstanceItem {
  return {
    id: inst.id,
    name: inst.name,
    label: getInstanceDisplayName(types, inst),
    typeName: types.find(t => t.key === inst.platform_type)?.name || inst.platform_type,
    platform_type: inst.platform_type,
  }
}

// ── 平台图标字形（[ZC-10] 从 InstanceMenu 上收为单一规则源，管理页"当前目标"卡共用）──
// 返回 'folder' 表示渲染文件夹线稿 SVG，否则是直接显示的字符；底色在 themes/*.css 的 --plat-*
export const PLAT_GLYPHS: Record<string, string> = {
  local: 'folder',
  notion: 'N',
  flowus: 'F',
  lark: '飞',
}
export function platGlyph(type: string): string {
  return PLAT_GLYPHS[type] ?? type.charAt(0).toUpperCase()
}