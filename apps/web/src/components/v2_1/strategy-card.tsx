import { BarChart3, CircleAlert, Database, ShieldCheck } from 'lucide-react'
import type { ReactNode } from 'react'
import { Link } from 'react-router'

import type { StrategyCatalogEntry } from '@/api/types'

type StrategyCardProps = {
  strategy: StrategyCatalogEntry
  analysisHref: string
  onOpenAnalysis?: () => void
  variant?: 'full' | 'compact'
  children?: ReactNode
}

export function StrategyCard({ strategy, analysisHref, onOpenAnalysis, variant = 'full', children }: StrategyCardProps) {
  const research = researchFacts(strategy)
  return (
    <article className="relative flex flex-col overflow-hidden rounded-[1.35rem] border border-slate-200 bg-white transition-[border-color,box-shadow,transform] duration-300 hover:-translate-y-0.5 hover:border-[#b8d5c6] hover:shadow-[0_10px_28px_rgba(16,32,40,0.08)] motion-reduce:transition-none motion-reduce:hover:translate-y-0">
      <Link
        to={analysisHref}
        aria-label={`查看${strategy.name}的直观分析`}
        onClick={onOpenAnalysis}
        className="group flex flex-1 flex-col p-5 text-left outline-none focus-visible:ring-3 focus-visible:ring-inset focus-visible:ring-[#8eb7a3]"
      >
        <div className="flex w-full items-start justify-between gap-3">
          <span className="rounded-full bg-[#e5eff4] px-2.5 py-1 text-xs font-medium text-[#294f60]">{riskLabel(strategy.risk)}</span>
          <span className="text-xs font-medium text-slate-400">{strategy.policy.id}@{strategy.policy.version}</span>
        </div>
        <h2 className="mt-5 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{strategy.name}</h2>
        <p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">{strategy.summary}</p>
        {variant === 'full' ? <div className="mt-5 rounded-xl bg-[#f6f8f6] p-3.5 text-sm leading-6 text-slate-700 transition-colors"><span className="font-medium text-[#102028]">它会怎么做：</span>{strategy.rule}</div> : null}
        <div className="mt-5 grid w-full grid-cols-2 gap-3 border-t border-slate-100 pt-4 text-sm">
          <div>
            <p className="flex items-center gap-1.5 text-xs text-slate-500"><Database className="size-3.5" />研究范围</p>
            <p className="mt-1 font-medium text-[#102028]">{research.scope}</p>
          </div>
          <div>
            <p className="flex items-center gap-1.5 text-xs text-slate-500"><ShieldCheck className="size-3.5" />可用状态</p>
            <p className="mt-1 font-medium text-[#102028]">{research.status}</p>
          </div>
        </div>
        <p className="mt-4 flex gap-2 text-xs leading-5 text-slate-500"><CircleAlert className="mt-0.5 size-3.5 shrink-0" />{strategy.limitation}</p>
        <span className="mt-5 inline-flex items-center gap-2 self-start rounded-full border border-[#b8d5c6] bg-white px-4 py-2.5 text-sm font-medium text-[#245a49] transition-colors group-hover:bg-[#e6f1eb]">
          <BarChart3 className="size-4" />查看直观分析
        </span>
      </Link>
      {children ? <div className="flex justify-end border-t border-slate-100 p-5 pt-4">{children}</div> : null}
    </article>
  )
}

function riskLabel(risk: StrategyCatalogEntry['risk']): string {
  return risk === 'stable' ? '稳健' : '平衡'
}

function researchFacts(strategy: StrategyCatalogEntry): { scope: string; status: string } {
  if (strategy.research_status === 'reference') {
    return { scope: '无需行情指标', status: '研究基准' }
  }
  const assetCount = strategy.research?.assets.length ?? 0
  return {
    scope: assetCount > 0 ? `${assetCount} 组指数序列` : '等待完整数据',
    status: strategy.adoptable ? '已通过准入' : '暂不可采用',
  }
}
