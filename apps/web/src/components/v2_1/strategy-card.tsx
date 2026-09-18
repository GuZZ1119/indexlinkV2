import { Check, CircleAlert, Eye } from 'lucide-react'
import type { ReactNode } from 'react'

import type { ConsumerStrategy } from '@/features/v2_1/model'
import { cn } from '@/lib/utils'

export function StrategyCard({ strategy, selected, onSelect, variant = 'full', children }: { strategy: ConsumerStrategy; selected: boolean; onSelect?: (id: ConsumerStrategy['id']) => void; variant?: 'full' | 'compact'; children?: ReactNode }) {
  return (
    <article className={cn('relative flex flex-col overflow-hidden rounded-[1.35rem] border bg-white transition-[border-color,background-color,box-shadow,transform] duration-300', selected ? 'border-[#2d6a57] bg-[#f2f8f5] shadow-[0_14px_34px_rgba(45,106,87,0.14)] ring-1 ring-[#b8d5c6]' : 'border-slate-200 hover:-translate-y-0.5 hover:border-[#b8d5c6] hover:shadow-[0_10px_28px_rgba(16,32,40,0.08)]')}>
      <button
        type="button"
        aria-pressed={selected}
        aria-label={`查看${strategy.name}`}
        onClick={() => onSelect?.(strategy.id)}
        className="group flex flex-1 flex-col p-5 text-left outline-none focus-visible:ring-3 focus-visible:ring-inset focus-visible:ring-[#8eb7a3]"
      >
        <div className="flex w-full items-start justify-between gap-3">
          <span className="rounded-full bg-[#e5eff4] px-2.5 py-1 text-xs font-medium text-[#294f60]">{strategy.risk}</span>
          {selected && <span className="inline-flex items-center gap-1 text-xs font-medium text-[#2d6a57]"><Check className="size-3.5" />正在查看</span>}
        </div>
        <h2 className="mt-5 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{strategy.name}</h2>
        <p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">{strategy.summary}</p>
        {variant === 'full' && <div className={cn('mt-5 rounded-xl p-3.5 text-sm leading-6 text-slate-700 transition-colors', selected ? 'bg-white/75' : 'bg-[#f6f8f6]')}><span className="font-medium text-[#102028]">它会怎么做：</span>{strategy.rule}</div>}
        <div className="mt-5 grid w-full grid-cols-2 gap-3 border-t border-slate-100 pt-4 text-sm">
          <div><p className="text-xs text-slate-500">历史年化</p><p className="mt-1 font-medium text-[#102028]">{strategy.annualizedReturn}</p></div>
          <div><p className="text-xs text-slate-500">最大回撤</p><p className="mt-1 font-medium text-[#102028]">{strategy.maxDrawdown}</p></div>
        </div>
        <p className="mt-4 flex gap-2 text-xs leading-5 text-slate-500"><CircleAlert className="mt-0.5 size-3.5 shrink-0" />{strategy.limitation}</p>
        <span className={cn('mt-5 inline-flex items-center gap-2 self-start rounded-full px-4 py-2.5 text-sm font-medium transition-colors', selected ? 'bg-[#2d6a57] text-white' : 'border border-[#b8d5c6] bg-white text-[#245a49] group-hover:bg-[#e6f1eb]')}>
          {selected ? <Check className="size-4" /> : <Eye className="size-4" />}{selected ? '正在查看' : '查看这个策略'}
        </span>
      </button>
      {children && <div className="mt-4 border-t border-slate-100 pt-4">{children}</div>}
    </article>
  )
}
