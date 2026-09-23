import { BarChart, LineChart, ScatterChart } from 'echarts/charts'
import { AriaComponent, DataZoomComponent, GridComponent, TooltipComponent } from 'echarts/components'
import { getInstanceByDom, init, use as registerEChartsModules, type EChartsCoreOption } from 'echarts/core'
import { SVGRenderer } from 'echarts/renderers'
import { useEffect, useRef } from 'react'

registerEChartsModules([
  LineChart,
  BarChart,
  ScatterChart,
  AriaComponent,
  DataZoomComponent,
  GridComponent,
  TooltipComponent,
  SVGRenderer,
])

interface InteractiveChartProps {
  option: EChartsCoreOption
  ariaLabel: string
  className?: string
}

/** Render an ECharts option while keeping resize and disposal inside one component boundary. */
export function InteractiveChart({ option, ariaLabel, className = 'h-full w-full' }: InteractiveChartProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container || import.meta.env.MODE === 'test') return

    const chart = init(container, undefined, { renderer: 'svg' })
    chart.setOption(option, { notMerge: true })

    const resize = () => chart.resize()
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(resize)
    observer?.observe(container)
    if (!observer) window.addEventListener('resize', resize)

    return () => {
      observer?.disconnect()
      if (!observer) window.removeEventListener('resize', resize)
      getInstanceByDom(container)?.dispose()
    }
  }, [option])

  return <div ref={containerRef} className={className} role="img" aria-label={ariaLabel} />
}
