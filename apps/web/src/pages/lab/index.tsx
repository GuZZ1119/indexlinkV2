import { Bot, Box, CheckCircle2, ChevronRight, Database, KeyRound, PlugZap, ShieldCheck } from 'lucide-react'
import { useState } from 'react'

import { PageHeading } from '@/components/v2_1/page-heading'
import { LegacyReplayPanel } from '@/components/v2_1/legacy-replay-panel'
import { connectionLabel, type LabConnectionState } from '@/features/v2_1/model'

const integrations = [
  { id: 'docker', icon: Box, title: '本地运行环境', description: 'Docker Compose、SQLite 与本机数据目录。适合希望自己掌控数据的人。', status: 'local-only' as const },
  { id: 'moomoo', icon: PlugZap, title: 'Moomoo / OpenD', description: '未来可读取模拟账户、生成待确认的订单草稿。此版本不提交真实订单。', status: 'not-configured' as const },
  { id: 'qwen', icon: Bot, title: 'Qwen AI', description: '为新闻与策略解释提供受限候选，不参与自动交易或修改策略规则。', status: 'not-configured' as const },
  { id: 'data', icon: Database, title: '市场数据', description: '连接你认可的数据来源，并在每份回测中保留版本与假设说明。', status: 'not-configured' as const },
] as const

export default function LabPage() {
  const [openedIntegration, setOpenedIntegration] = useState<(typeof integrations)[number]['id'] | null>(null)

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="高级实验室" title="把复杂配置，留给想深入的人" description="这里不会改变你的主计划，也不会默认连接任何账户。每项能力都需要你在本机明确配置并确认。" />

      <section className="grid gap-4 md:grid-cols-2">{integrations.map((integration) => <IntegrationCard key={integration.id} integration={integration} active={openedIntegration === integration.id} onOpen={() => setOpenedIntegration(integration.id)} onClose={() => setOpenedIntegration(null)} />)}</section>

      <section className="rounded-[1.35rem] border border-[#eadfc4] bg-[#fcf8ed] p-5 sm:p-6"><div className="flex gap-3"><ShieldCheck className="mt-0.5 size-5 shrink-0 text-[#9a6d20]" /><div><h2 className="font-semibold text-[#102028]">执行权限仍保持关闭</h2><p className="mt-1.5 max-w-3xl text-sm leading-6 text-slate-600">V2.1 的高级配置只为后续连接预留入口。当前页面不保存密钥、不验证账号、不提交模拟或真实订单；请在本机环境变量或 Docker 配置中完成真实配置。</p></div></div></section>

      <LegacyReplayPanel />

      <section className="grid gap-4 md:grid-cols-3"><Guardrail icon={<KeyRound />} title="密钥不进浏览器" text="API 密钥只应由本地服务端读取。" /><Guardrail icon={<CheckCircle2 />} title="每次执行都确认" text="未来即使接入券商，也应展示订单草稿与确认步骤。" /><Guardrail icon={<ShieldCheck />} title="先模拟，后执行" text="真实账户接入前，先完成可复核的模拟验证。" /></section>
    </div>
  )
}

function IntegrationCard({ integration, active, onOpen, onClose }: { integration: (typeof integrations)[number]; active: boolean; onOpen: () => void; onClose: () => void }) {
  const Icon = integration.icon
  return <article className={`rounded-[1.35rem] border bg-white p-5 transition-colors ${active ? 'border-[#2d6a57]' : 'border-slate-200 hover:border-slate-300'}`}><div className="flex items-start justify-between gap-4"><span className="grid size-10 place-items-center rounded-xl bg-[#e5eff4] text-[#294f60]"><Icon className="size-5" /></span><span className={`rounded-full px-2.5 py-1 text-xs font-medium ${integration.status === 'local-only' ? 'bg-[#e6f2eb] text-[#2d6a57]' : 'bg-slate-100 text-slate-500'}`}>{connectionLabel(integration.status as LabConnectionState)}</span></div><h2 className="mt-5 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{integration.title}</h2><p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">{integration.description}</p>{active ? <div aria-live="polite" className="mt-5 rounded-xl bg-[#f6f8f6] p-4"><p className="text-sm font-medium text-[#102028]">配置预览</p><div className="mt-3 font-mono text-xs leading-6 text-slate-600"><p># 此壳子不写入任何配置</p><p>INDEXLINK_{integration.id.toUpperCase()}_ENABLED=false</p><p>请通过 Docker 环境变量或本地 .env 在服务端完成配置。</p></div><button type="button" onClick={onClose} className="mt-3 text-sm font-medium text-[#2d6a57]">收起预览</button></div> : <button type="button" onClick={onOpen} className="mt-5 inline-flex items-center gap-1 text-sm font-medium text-[#2d6a57]">打开配置预览 <ChevronRight className="size-3.5" /></button>}</article>
}

function Guardrail({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) { return <div className="rounded-[1.2rem] bg-[#eef4f1] p-5 text-sm"><span className="text-[#2d6a57]">{icon}</span><h3 className="mt-3 font-semibold text-[#102028]">{title}</h3><p className="mt-1.5 leading-6 text-slate-600">{text}</p></div> }
