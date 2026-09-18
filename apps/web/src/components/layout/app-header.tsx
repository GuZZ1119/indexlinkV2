import { Languages, Menu } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'

export function AppHeader() {
  const { t, i18n } = useTranslation()

  const toggleLanguage = () => {
    void i18n.changeLanguage(i18n.language.startsWith('zh') ? 'en' : 'zh')
  }

  return (
    <header className="sticky top-0 z-20 flex h-[4.5rem] shrink-0 items-center gap-3 border-b border-slate-200/80 bg-[#f6f8f6]/90 px-5 backdrop-blur md:px-8">
      <Menu className="size-5 text-slate-500 lg:hidden" aria-hidden="true" />
      <Link to="/personal" className="flex items-center gap-2.5">
        <span className="grid size-8 place-items-center rounded-[0.7rem] bg-[#102028] text-sm font-semibold text-white">I</span>
        <span className="text-[1rem] font-semibold tracking-[-0.04em] text-[#102028]">IndexLink</span>
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

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="gap-2 px-2">
              <Avatar className="size-7">
                <AvatarFallback className="text-xs">IL</AvatarFallback>
              </Avatar>
              <span className="hidden text-sm sm:inline">{t('live.localDemo')}</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-52">
            <DropdownMenuLabel>
              <div className="flex flex-col">
                <span>{t('live.localDemo')}</span>
              <span className="text-xs font-normal text-muted-foreground">{t('live.localOnly')}</span>
              </div>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem>{t('header.profile')}</DropdownMenuItem>
            <DropdownMenuItem>{t('header.settings')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive">
              {t('header.signOut')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  )
}
