import { beforeEach } from 'vitest'
import i18n from './index'

// Existing UI fixtures use Chinese. Each test can explicitly opt into English.
beforeEach(async () => { await i18n.changeLanguage('zh') })
