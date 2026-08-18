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