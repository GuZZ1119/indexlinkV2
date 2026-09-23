import type { BacktestExecutionPoint, BacktestMarketPoint, DynamicBacktestSeries, StrategyCatalogEntry } from '@/api/types'
import { strategyAnalysisColor } from '@/features/v2_1/model'

export interface MarketExecution extends BacktestExecutionPoint {
  strategyId: string
  strategyName: string
  color: string
}

export interface MarketChartPoint extends BacktestMarketPoint {
  executions: MarketExecution[]
}

/** Join server-returned executions to their matching daily close without inventing points. */
export function buildMarketChartData(marketPoints: BacktestMarketPoint[], series: DynamicBacktestSeries[], catalogById: Map<string, StrategyCatalogEntry>): MarketChartPoint[] {
  const pointsByDate = new Map(marketPoints.map((point) => [point.date, { ...point, executions: [] as MarketExecution[] }]))
  for (const item of series) {
    for (const execution of item.execution_points) {
      const point = pointsByDate.get(execution.date)
      if (!point) continue
      point.executions.push({
        ...execution,
        strategyId: item.strategy_id,
        strategyName: catalogById.get(item.strategy_id)?.name ?? item.strategy_name ?? item.strategy_id,
        color: strategyAnalysisColor(item.strategy_id),
      })
    }
  }
  return [...pointsByDate.values()]
}
