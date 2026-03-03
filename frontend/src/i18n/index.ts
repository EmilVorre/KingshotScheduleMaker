import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import en from '../locales/en.json'
import ko from '../locales/ko.json'
import zh from '../locales/zh.json'
import ja from '../locales/ja.json'
import es from '../locales/es.json'
import de from '../locales/de.json'
import fr from '../locales/fr.json'

export const SUPPORTED_LANGUAGES = ['en', 'ko', 'zh', 'ja', 'es', 'de', 'fr'] as const
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number]

const LANGUAGE_LABELS: Record<SupportedLanguage, string> = {
  en: 'English',
  ko: '한국어',
  zh: '中文',
  ja: '日本語',
  es: 'Español',
  de: 'Deutsch',
  fr: 'Français',
}

export const LANGUAGE_OPTIONS = SUPPORTED_LANGUAGES.map((code) => ({
  value: code,
  label: LANGUAGE_LABELS[code],
}))

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    ko: { translation: ko },
    zh: { translation: zh },
    ja: { translation: ja },
    es: { translation: es },
    de: { translation: de },
    fr: { translation: fr },
  },
  lng: typeof window !== 'undefined' ? localStorage.getItem('form_language') || 'en' : 'en',
  fallbackLng: 'en',
  supportedLngs: ['en', 'ko', 'zh', 'ja', 'es', 'de', 'fr'],
  load: 'languageOnly',
  interpolation: {
    escapeValue: false,
  },
})

i18n.on('languageChanged', (lng) => {
  if (typeof window !== 'undefined') {
    localStorage.setItem('form_language', lng)
  }
})

export default i18n
