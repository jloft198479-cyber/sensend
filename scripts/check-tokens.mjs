#!/usr/bin/env node
/**
 * Sensend 设计 token 守门脚本
 * 规则来源：docs/Sensend-界面设计规范与整改方案-v1.0-20261116.docx
 *   §4.2 禁止颜色字面量（主题文件除外）
 *   §4.3 字阶/字重白名单
 *   §4.4 间距一律走 --space-*
 *   §4.5 圆角只允许 token 或 0 / 50%
 *   §4.6 阴影只允许 token
 *   §4.8 z-index 只允许 token
 *
 * 用法：npm run check:tokens    退出码 1 = 有违规
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const SRC = path.join(ROOT, 'src')
const FONT_SIZES = new Set(['0', '11px', '12px', '13px', '15px', '17px'])
const FONT_WEIGHTS = new Set(['400', '500', '600', '700'])
const COLOR = /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/

function walk(dir) {
  const out = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) out.push(...walk(full))
    else out.push(full)
  }
  return out
}

const problems = []
const files = walk(SRC).filter(
  (f) => /\.(vue|css)$/.test(f) && !f.includes(`${path.sep}themes${path.sep}`),
)

for (const file of files) {
  const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/)
  lines.forEach((raw, i) => {
    const line = raw.trim()
    if (!line || line.startsWith('/*') || line.startsWith('*') || line.startsWith('//')) return
    // 自定义属性定义行（--xxx: ...）是 token 源头，跳过
    if (line.startsWith('--')) return
    const where = `${path.relative(ROOT, file)}:${i + 1}`
    const at = (msg) => problems.push(`${where}  ${msg}  →  ${line}`)

    if (COLOR.test(line)) at('禁止颜色字面量（hex/rgb/hsl），请用 token')

    const z = line.match(/z-index\s*:\s*([^;]+)/)
    if (z && !z[1].trim().startsWith('var(')) at('z-index 只能引用 --z-* token')

    const shadow = line.match(/box-shadow\s*:\s*([^;]+)/)
    if (shadow) {
      const v = shadow[1].trim()
      if (!v.startsWith('var(') && v !== 'none') at('box-shadow 只能引用 --shadow-* token')
    }

    const radius = line.match(/border-radius\s*:\s*([^;]+)/)
    if (radius) {
      const parts = radius[1].trim().split(/\s+/).filter(Boolean)
      const ok = parts.every((p) => p === '0' || p === '50%' || p.startsWith('var('))
      if (!ok) at('border-radius 只能用 token / 50% / 0')
    }

    const size = line.match(/font-size\s*:\s*([^;]+)/)
    if (size) {
      const v = size[1].trim()
      if (v.endsWith('px') && !FONT_SIZES.has(v)) at('字号不在字阶白名单（11/12/13/15/17）')
    }

    const weight = line.match(/font-weight\s*:\s*([^;!]+)/)
    if (weight) {
      const v = weight[1].trim()
      if (!FONT_WEIGHTS.has(v)) at('字重只允许 400/500/600/700')
    }

    const spacing = line.match(/(?:padding|margin|gap)(?:-[a-z]+)?\s*:\s*([^;]+)/)
    if (spacing) {
      const v = spacing[1].trim()
      if (/\d+px/.test(v)) at('间距必须用 --space-* token（禁止 px 字面量）')
    }
  })
}

if (problems.length) {
  console.log(`✗ 设计 token 检查未通过（${problems.length} 处）\n`)
  for (const p of problems) console.log(`  ${p}`)
  process.exit(1)
}
console.log(`✓ 设计 token 检查通过（扫描 ${files.length} 个文件，无颜色/间距/圆角/阴影/字号/字重/z-index 字面量）`)
