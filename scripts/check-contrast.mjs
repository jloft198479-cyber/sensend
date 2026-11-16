#!/usr/bin/env node
/**
 * Sensend 色板对比度校验（WCAG 2.1）
 * 规范：docs/Sensend-界面设计规范与整改方案-v1.0-20261116.docx §4.2.3 / 附录 B
 *
 * 用法：npm run check:contrast   （或 node scripts/check-contrast.mjs）
 * 退出码：全部达标且四主题变量同构 = 0；任一不达标/缺项 = 1
 *
 * 只读取主题 CSS，不依赖浏览器；改色后跑一遍即可回归。
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const THEMES = [
  { id: 'light', name: '草绿', file: 'src/styles/themes/light.css' },
  { id: 'dark', name: '暗夜', file: 'src/styles/themes/dark.css' },
  { id: 'wenyi', name: '秋珀', file: 'src/styles/themes/wenyi.css' },
  { id: 'tech', name: '魅蓝', file: 'src/styles/themes/tech.css' },
  { id: 'ink', name: '墨白', file: 'src/styles/themes/ink.css' },
]

/** 需要校验的色对（fg/bg 为 CSS 变量名，min 为 WCAG 门槛） */
const PAIRS = [
  { label: '主文字/主面', fg: '--fg', bg: '--bg', min: 4.5 },
  { label: '次要/主面', fg: '--fg-secondary', bg: '--bg', min: 4.5 },
  { label: '辅助/主面', fg: '--muted', bg: '--bg', min: 4.5 },
  { label: '辅助/次面', fg: '--muted', bg: '--bg-page', min: 4.5 },
  { label: '占位/输入底', fg: '--placeholder', bg: '--gray-input-bg', min: 4.5 },
  { label: '可读色/主面', fg: '--accent-strong', bg: '--bg', min: 4.5 },
  { label: '可读色/淡底', fg: '--accent-strong', bg: '--accent-light', min: 4.5 },
  { label: '按钮字/底', fg: '--fg-on-accent', bg: '--accent-fill', min: 4.5 },
  { label: '描边/输入底', fg: '--control-border', bg: '--gray-input-bg', min: 3.0 },
  { label: '危险字/底', fg: '--danger', bg: '--danger-bg', min: 4.5 },
  { label: '成功字/底', fg: '--success-text', bg: '--success-bg', min: 4.5 },
  { label: '警告字/底', fg: '--warning-text', bg: '--warning-bg', min: 4.5 },
]

function readVars(file) {
  const css = fs.readFileSync(file, 'utf8')
  const vars = new Map()
  for (const m of css.matchAll(/--([\w-]+)\s*:\s*([^;]+);/g)) {
    vars.set(`--${m[1]}`, m[2].trim())
  }
  return vars
}

/** 解析 #rgb / #rrggbb / rgb() / rgba()，统一按 0-255 + alpha 0-1 返回 */
function parseColor(raw) {
  const v = raw.trim().toLowerCase()
  const hex = v.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/)
  if (hex) {
    const h = hex[1]
    const full = h.length === 3 ? h.split('').map((c) => c + c).join('') : h
    return {
      r: parseInt(full.slice(0, 2), 16),
      g: parseInt(full.slice(2, 4), 16),
      b: parseInt(full.slice(4, 6), 16),
      a: 1,
    }
  }
  const rgb = v.match(/^rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\)$/)
  if (rgb) {
    return { r: +rgb[1], g: +rgb[2], b: +rgb[3], a: rgb[4] === undefined ? 1 : +rgb[4] }
  }
  return null
}

/** 半透明色按 sRGB 通道与底色合成（附录 B.2） */
function composite(fg, bg) {
  const a = fg.a
  return {
    r: fg.r * a + bg.r * (1 - a),
    g: fg.g * a + bg.g * (1 - a),
    b: fg.b * a + bg.b * (1 - a),
    a: 1,
  }
}

function luminance({ r, g, b }) {
  const lin = (c) => {
    const s = c / 255
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

function contrast(fgRaw, bgRaw, vars) {
  const parse = (raw, name) => {
    const c = parseColor(raw)
    if (!c) throw new Error(`无法解析颜色 ${name} = ${raw}`)
    return c
  }
  const bgBase = parse(vars.get('--bg'), '--bg')
  let bg = parse(bgRaw, 'bg')
  if (bg.a < 1) bg = composite(bg, bgBase)
  let fg = parse(fgRaw, 'fg')
  if (fg.a < 1) fg = composite(fg, bg)
  const l1 = luminance(fg)
  const l2 = luminance(bg)
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05)
}

const REQUIRED = new Set(PAIRS.flatMap((p) => [p.fg, p.bg]))
const rows = []
const problems = []
let referenceKeys = null

for (const theme of THEMES) {
  const file = path.join(ROOT, theme.file)
  if (!fs.existsSync(file)) {
    problems.push(`主题文件缺失：${theme.file}`)
    continue
  }
  const vars = readVars(file)
  const keys = [...vars.keys()].sort()

  if (referenceKeys === null) {
    referenceKeys = keys
  } else if (keys.length !== referenceKeys.length || keys.some((k, i) => k !== referenceKeys[i])) {
    const missing = referenceKeys.filter((k) => !vars.has(k))
    const extra = keys.filter((k) => !referenceKeys.includes(k))
    problems.push(
      `${theme.name} 主题变量不同构：缺 [${missing.join(', ') || '无'}]，多 [${extra.join(', ') || '无'}]`,
    )
  }

  for (const name of REQUIRED) {
    if (!vars.has(name)) problems.push(`${theme.name} 缺少变量 ${name}`)
  }

  for (const pair of PAIRS) {
    if (!vars.has(pair.fg) || !vars.has(pair.bg)) continue
    const ratio = contrast(vars.get(pair.fg), vars.get(pair.bg), vars)
    const pass = ratio >= pair.min - 1e-9
    rows.push({ theme: theme.name, label: pair.label, ratio, min: pair.min, pass })
    if (!pass) {
      problems.push(
        `${theme.name} ${pair.label}：${ratio.toFixed(2)} < ${pair.min}（${pair.fg} / ${pair.bg}）`,
      )
    }
  }
}

const pad = (s, n) => String(s).padEnd(n, ' ')
console.log('=== Sensend 色板对比度校验（WCAG 2.1）===\n')
console.log(pad('主题', 8) + pad('组合', 18) + pad('实测', 8) + pad('门槛', 8) + '结果')
for (const r of rows) {
  console.log(
    pad(r.theme, 8) +
      pad(r.label, 18) +
      pad(r.ratio.toFixed(2), 8) +
      pad(r.min.toFixed(1), 8) +
      (r.pass ? 'PASS' : 'FAIL'),
  )
}

console.log(`\n主题变量数：${THEMES.map((t) => {
  const f = path.join(ROOT, t.file)
  return fs.existsSync(f) ? `${t.name}=${readVars(f).size}` : `${t.name}=缺失`
}).join('  ')}`)

if (problems.length) {
  console.log('\n问题：')
  for (const p of problems) console.log(`  - ${p}`)
  console.log('\n结果：不通过')
  process.exit(1)
}
console.log('\n结果：全部通过')
