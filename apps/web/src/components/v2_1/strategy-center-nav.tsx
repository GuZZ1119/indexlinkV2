import { BarChart3, BookOpen } from 'lucide-react'
import { NavLink } from 'react-router'

const items = [
  { to: '/strategy-center', label: '策略库', icon: BookOpen, end: true },
  { to: '/strategy-analysis', label: '策略分析', icon: BarChart3, end: false },
] as const

/** Local navigation keeps discovery and analysis together without adding a trading-terminal layer. */
export function StrategyCenterNav() {
  return (
    <nav aria-label="策略中心导航" className="flex w-fit rounded-full border border-slate-200 bg-white p-1 shadow-sm">
      {items.map((item) => <NavLink key={item.to} to={item.to} end={item.end} className={({ isActive }) => `inline-flex items-center gap-2 rounded-full px-3.5 py-2 text-sm font-medium transition-colors ${isActive ? 'bg-[#102028] text-white' : 'text-slate-500 hover:bg-slate-100 hover:text-[#102028]'}`}><item.icon className="size-3.5" />{item.label}</NavLink>)}
    </nav>
  )
}
