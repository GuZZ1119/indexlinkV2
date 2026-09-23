import { BarChart3, BookOpen, ClipboardList, FlaskConical, UserRound, WandSparkles } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'

import { SheetClose } from '@/components/ui/sheet'

const navItems = [
  { to: '/personal', key: 'nav.personal', icon: UserRound, nested: false },
  { to: '/plans', key: 'nav.myPlans', icon: ClipboardList, nested: true },
  { to: '/strategy-center', key: 'nav.strategiesCenter', icon: BookOpen, nested: false },
  { to: '/strategy-builder', key: 'nav.strategyBuilder', icon: WandSparkles, nested: true },
  { to: '/strategy-analysis', key: 'nav.strategyAnalysis', icon: BarChart3, nested: true },
  { to: '/lab', key: 'nav.lab', icon: FlaskConical, nested: false },
] as const

export function AppNavigation({ mobile = false }: { mobile?: boolean }) {
  const { t } = useTranslation()

  return (
    <div className="space-y-1">
      {navItems.map((item) => {
        const link = <NavLink key={item.to} to={item.to} className={({ isActive }) => `flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-colors ${item.nested && !mobile ? 'ml-4 text-[0.82rem]' : ''} ${isActive ? 'bg-[#102028] text-white' : 'text-slate-600 hover:bg-slate-100 hover:text-[#102028]'}`}><item.icon className="size-4" /><span>{t(item.key)}</span></NavLink>
        return mobile ? <SheetClose asChild key={item.to}>{link}</SheetClose> : link
      })}
    </div>
  )
}

export function AppSidebar() {
  const { t } = useTranslation()

  return (
    <nav aria-label={t('nav.primary')} className="sticky top-[4.5rem] hidden h-[calc(100svh-4.5rem)] w-56 shrink-0 border-r border-slate-200/80 bg-white/45 px-4 py-7 lg:block">
      <p className="px-3 text-xs font-medium text-slate-400">{t('nav.workspace')}</p>
      <div className="mt-3"><AppNavigation /></div>
      <p className="absolute inset-x-7 bottom-7 text-xs leading-5 text-slate-400">
        {t('nav.localFirst')}<br />{t('nav.localData')}
      </p>
    </nav>
  )
}
