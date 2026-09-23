import { Languages, Menu } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Button } from '@/components/ui/button'
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet'

import { AppNavigation } from './app-sidebar'

export function AppHeader() {
  const { t, i18n } = useTranslation()

  const toggleLanguage = () => {
    void i18n.changeLanguage(i18n.language.startsWith('zh') ? 'en' : 'zh')
  }

  return (
    <header className="sticky top-0 z-20 flex h-[4.5rem] shrink-0 items-center gap-3 border-b border-slate-200/80 bg-[#f6f8f6]/90 px-5 backdrop-blur md:px-8">
      <Sheet>
        <SheetTrigger asChild>
          <Button variant="ghost" size="icon" className="lg:hidden" aria-label={t('header.toggleSidebar')}>
            <Menu className="size-5" />
          </Button>
        </SheetTrigger>
        <SheetContent side="left" className="w-[18rem] bg-[#f6f8f6] p-0">
          <SheetHeader className="border-b border-slate-200 px-6 py-5 text-left">
            <SheetTitle>{t('common.appName')}</SheetTitle>
            <SheetDescription>{t('live.localOnly')}</SheetDescription>
          </SheetHeader>
          <nav aria-label={t('nav.primary')} className="px-4 py-5">
            <AppNavigation mobile />
          </nav>
        </SheetContent>
      </Sheet>
      <Link to="/personal" className="flex min-w-0 items-center" aria-label={t('common.appName')}>
        <img
          src="/logo.png"
          alt=""
          className="h-auto w-[7.75rem] shrink-0 object-contain sm:w-[8.75rem]"
        />
      </Link>

      <div className="ml-auto flex items-center gap-1.5">
        <Button
          variant="ghost" size="sm"
          onClick={toggleLanguage}
          aria-label={t('header.switchLanguage')}
        >
          <Languages className="size-4" />
          <span className="text-xs font-medium uppercase">
            {i18n.language.startsWith('zh') ? '中' : 'EN'}
          </span>
        </Button>
      </div>
    </header>
  )
}
