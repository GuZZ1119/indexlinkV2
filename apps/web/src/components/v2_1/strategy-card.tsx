import { ArrowUpRight, Check, CircleAlert } from 'lucide-react'
import type { ReactNode } from 'react'

import type { ConsumerStrategy } from '@/features/v2_1/model'
import { cn } from '@/lib/utils'

export function StrategyCard({ strategy, selected, onSelect, variant = 'full', children }: { strategy: ConsumerStrategy; selected: boolean; onSelect?: (id: ConsumerStrategy['id']) => void; variant?: 'full' | 'compact'; children?: ReactNode }) {
  return (
    <article className={cn('relative flex flex-col rounded-[1.35rem] border bg-white p-5 transition-colors', selected ? 'border-[#2d6a57] bg-[#f5faf7]' : 'border-slate-200 hover:border-slate-300')}>
      <div className="flex items-start justify-between gap-3">
        <span className="rounded-full bg-[#e5eff4] px-2.5 py-1 text-xs font-medium text-[#294f60]">{strategy.risk}</span>
        {selected && <span className="inline-flex items-center gap-1 text-xs font-medium text-[#2d6a57]"><Check className="size-3.5" />正在坚持</span>}
      </div>
      <h2 className="mt-5 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{strategy.name}</h2>
      <p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">{strategy.summary}</p>
      {variant === 'full' && <div className="mt-5 rounded-xl bg-[#f6f8f6] p-3.5 text-sm leading-6 text-slate-700"><span className="font-medium text-[#102028]">它会怎么做：</span>{strategy.rule}</div>}
      <div className="mt-5 grid grid-cols-2 gap-3 border-t border-slate-100 pt-4 text-sm">
        <div><p className="text-xs text-slate-500">历史年化</p><p className="mt-1 font-medium text-[#102028]">{strategy.annualizedReturn}</p></div>
        <div><p className="text-xs text-slate-500">最大回撤</p><p className="mt-1 font-medium text-[#102028]">{strategy.maxDrawdown}</p></div>
      </div>
      <p className="mt-4 flex gap-2 text-xs leading-5 text-slate-500"><CircleAlert className="mt-0.5 size-3.5 shrink-0" />{strategy.limitation}</p>
      {selected
        ? <p className="mt-5 inline-flex items-center gap-1 self-start text-sm font-medium text-[#2d6a57]"><Check className="size-3.5" />当前正在使用</p>
        : <button type="button" onClick={() => onSelect?.(strategy.id)} className="mt-5 inline-flex items-center gap-1 self-start text-sm font-medium text-[#2d6a57] outline-none hover:text-[#1f5444] focus-visible:rounded focus-visible:ring-2 focus-visible:ring-[#2d6a57] focus-visible:ring-offset-2">选用这个策略 <ArrowUpRight className="size-3.5" /></button>}
      {children && <div className="mt-4 border-t border-slate-100 pt-4">{children}</div>}
    </article>
  )
}
