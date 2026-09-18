import type { ReactNode } from 'react'

export function PageHeading({ eyebrow, title, description, action }: { eyebrow: string; title: string; description: string; action?: ReactNode }) {
  return (
    <header className="flex flex-col justify-between gap-5 border-b border-slate-200/80 pb-7 md:flex-row md:items-end">
      <div className="max-w-2xl">
        <p className="text-sm font-medium text-[#2d6a57]">{eyebrow}</p>
        <h1 className="mt-2 text-3xl font-semibold tracking-[-0.045em] text-[#102028] sm:text-4xl">{title}</h1>
        <p className="mt-3 max-w-xl text-[0.96rem] leading-7 text-slate-600">{description}</p>
      </div>
      {action}
    </header>
  )
}
