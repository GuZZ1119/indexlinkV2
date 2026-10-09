import { useTranslation } from 'react-i18next'

import i18n, { appLanguage } from './index'

/** Presentation only: never translate user text, API identifiers or audit evidence. */
export function uiText(source: string, values: Record<string, unknown> = {}): string {
  return i18n.t(source, { ...values, ns: 'ui', keySeparator: false, nsSeparator: false, defaultValue: source })
}

export function uiLocale(): string {
  return appLanguage(i18n.resolvedLanguage ?? i18n.language ?? 'zh') === 'zh' ? 'zh-CN' : 'en-US'
}

/** Subscribe without remounting the page or discarding its in-progress form. */
export function useUiLocale(): string {
  useTranslation('ui')
  return uiLocale()
}
