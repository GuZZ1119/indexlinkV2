import i18n from 'i18next'
import LanguageDetector from 'i18next-browser-languagedetector'
import { initReactI18next } from 'react-i18next'

import en from './locales/en'
import zh from './locales/zh'
import { uiEnglish } from './ui-messages'
import { catalogEnglish } from './catalog-messages'

const uiMessages = { ...catalogEnglish, ...uiEnglish }

export const SUPPORTED_LANGUAGES = ['zh', 'en'] as const
export type AppLanguage = (typeof SUPPORTED_LANGUAGES)[number]

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      zh: { translation: zh, ui: Object.fromEntries(Object.keys(uiMessages).map((source) => [source, source])) },
      en: { translation: en, ui: uiMessages },
    },
    fallbackLng: 'zh',
    supportedLngs: [...SUPPORTED_LANGUAGES],
    load: 'languageOnly',
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  })

/** Normalize a browser language to one of the application's supported locales. */
export function appLanguage(lang: string): AppLanguage {
  return lang.toLowerCase().startsWith('zh') ? 'zh' : 'en'
}

i18n.on('languageChanged', (language) => {
  document.documentElement.lang = appLanguage(language)
})
document.documentElement.lang = appLanguage(i18n.resolvedLanguage ?? i18n.language ?? 'zh')

export default i18n
