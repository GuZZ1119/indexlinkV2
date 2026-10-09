import type { StrategyCatalogEntry } from '@/api/types'
import { catalogEnglish } from './catalog-messages'
import { uiLocale, uiText } from './ui'

const rulePatterns = Object.keys(catalogEnglish).filter((key) => key.includes('{{')).map((source) => ({
  source,
  pattern: new RegExp(`^${source.split(/\{\{p\d+\}\}/).map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('(-?\\d+(?:\\.\\d+)?)')}$`),
}))

export function catalogText(source: string): string {
  if (uiLocale() === 'zh-CN') return source
  if (Object.hasOwn(catalogEnglish, source)) return uiText(source)
  for (const { source: template, pattern } of rulePatterns) {
    const match = source.match(pattern)
    if (match) return uiText(template, Object.fromEntries(match.slice(1).map((value, index) => [`p${index}`, value])))
  }
  // Translate only known official labels; keep unknown/provider text as its source.
  const translated = source.split(/（|）| · /).map((part) => Object.hasOwn(catalogEnglish, part) ? uiText(part) : part
    .replace(/(趋势|波动|动量|回撤|增长)(\d+)日/g, (_, kind: string, days: string) => `${uiText(kind)} ${days}d`)
    .replace(/(\d+(?:\/\d+)*)日/g, '$1d'))
  const original = source.split(/（|）| · /)
  return translated.some((part, index) => part !== original[index]) ? translated.join(source.includes('（') ? ' ' : ' · ').trim() : source
}

/** React Query select projection: cache and server audit snapshots remain canonical. */
export function localizeCatalog(entries: StrategyCatalogEntry[], locale = uiLocale()): StrategyCatalogEntry[] {
  if (locale === 'zh-CN') return entries
  return entries.map((entry) => entry.origin === 'personal' ? entry : ({
    ...entry,
    name: catalogText(entry.name), summary: catalogText(entry.summary), rule: catalogText(entry.rule), limitation: catalogText(entry.limitation),
    family: entry.family ? { ...entry.family, name: catalogText(entry.family.name), description: catalogText(entry.family.description) } : undefined,
    preset: entry.preset ? { ...entry.preset, name: catalogText(entry.preset.name) } : undefined,
    source: entry.source ? { ...entry.source, adaptation: catalogText(entry.source.adaptation) } : undefined,
    tags: entry.tags?.map(catalogText),
  }))
}
