import { Bot, Box, CheckCircle2, ChevronRight, Database, KeyRound, Loader2, PlugZap, ShieldCheck, Trash2 } from 'lucide-react'
import { useState, type FormEvent } from 'react'

import { useAiProviders, useClearSessionAiProvider, useClearSessionOpenD, useConfigureSessionAiProvider, useConfigureSessionOpenD, useRuntimeStatus, useTestSessionAiProvider } from '@/api/queries'
import type { ConfigureSessionAiProviderRequest, SessionAiProbeStatus } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PageHeading } from '@/components/v2_1/page-heading'

type IntegrationId = 'docker' | 'moomoo' | 'ai' | 'data'

const integrations: Array<{ id: IntegrationId; icon: typeof Box; title: string; description: string }> = [
  { id: 'docker', icon: Box, title: '本地运行环境', description: 'Docker Compose、SQLite 与本机数据目录。它们决定服务如何启动，不会在页面中热切换。' },
  { id: 'moomoo', icon: PlugZap, title: 'Moomoo / OpenD', description: '输入本机 OpenD 地址后，可立即作为真实日线和回测来源；网页不会开启模拟券商或下单权限。' },
  { id: 'ai', icon: Bot, title: 'QwenCloud / 百炼 / GPT / Claude / DeepSeek', description: '在这里输入你自己的 API。QwenCloud 与阿里云百炼使用不同地址和 Key，不会混用。' },
  { id: 'data', icon: Database, title: '市场数据', description: '当前真实回测复用 OpenD 日线适配器，并为每次结果保留来源、时间范围和校验值。' },
]

const providerDefaults: Record<ConfigureSessionAiProviderRequest['provider'], string> = {
  qwen_cloud: 'qwen3.8-max',
  qwen: 'qwen-plus',
  gpt: 'gpt-6-astra',
  claude: 'claude-sonnet-5',
  deepseek: 'deepseek-chat',
}

const probeCopy: Record<SessionAiProbeStatus, { title: string; detail: string; className: string }> = {
  available: {
    title: 'AI 连接可用',
    detail: '供应商接受了当前 Key 和模型。生成草案或解释时仍会单独校验返回结构。',
    className: 'border-[#b9d8c8] bg-[#edf6f1] text-[#245b49]',
  },
  authentication_failed: {
    title: 'API Key 认证失败',
    detail: '请重新复制完整 Key，并确认 sk-ws- 选择 QwenCloud、百炼 Key 选择阿里云百炼。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  access_denied: {
    title: '账户权限或计费未开通',
    detail: '请检查工作区权限、Pay-As-You-Go 计费状态以及该模型是否已激活。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  model_unavailable: {
    title: '模型名称不可用',
    detail: '请核对模型 ID，并确认它已在当前供应商和工作区中启用。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  rate_limited: {
    title: '额度或调用频率受限',
    detail: '请检查账户余额与限流状态，稍后再手动验证。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  request_rejected: {
    title: '供应商拒绝了当前配置',
    detail: '模型可能尚未激活或不兼容当前接口；请先检查模型市场与账户设置。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  network_unavailable: {
    title: '连接超时或网络不可达',
    detail: 'QwenCloud 已使用 90 秒等待时间；若仍失败，请检查本机网络、代理与供应商状态。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  provider_unavailable: {
    title: '供应商暂时不可用',
    detail: '连接已保存，但供应商没有完成这次最小请求。请稍后重试。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
  response_invalid: {
    title: '供应商返回了无法识别的内容',
    detail: 'Key 与网络可能正常，但当前模型响应不符合兼容接口格式。',
    className: 'border-[#e7cf9f] bg-[#fff8e9] text-[#75511d]',
  },
}

export default function LabPage() {
  const [openedIntegration, setOpenedIntegration] = useState<IntegrationId | null>(null)

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="高级实验室" title="把复杂连接，留在一个清楚的地方" description="AI 与只读 OpenD 行情可以直接建立本次运行连接；任何能力都不会自动执行策略或下单。" />

      <section className="grid gap-4 md:grid-cols-2">
        {integrations.map((integration) => <IntegrationCard key={integration.id} integration={integration} active={openedIntegration === integration.id} onOpen={() => setOpenedIntegration(integration.id)} onClose={() => setOpenedIntegration(null)} />)}
      </section>

      <section className="rounded-[1.35rem] border border-[#eadfc4] bg-[#fcf8ed] p-5 sm:p-6"><div className="flex gap-3"><ShieldCheck className="mt-0.5 size-5 shrink-0 text-[#9a6d20]" /><div><h2 className="font-semibold text-[#102028]">密钥只停留在当前后端进程</h2><p className="mt-1.5 max-w-3xl text-sm leading-6 text-slate-600">从页面输入的 AI 密钥不会写入浏览器存储、SQLite、日志或配置文件。关闭或重启 Rust 后端后，这条连接自动消失。策略草案、回测解释和个人摘要仍须分别手动点击；新闻情绪只留在高级实验室。</p></div></div></section>

      <section className="grid gap-4 md:grid-cols-3"><Guardrail icon={<KeyRound />} title="只保存到内存" text="页面只接收输入，后端响应永远不回传密钥。" /><Guardrail icon={<CheckCircle2 />} title="每次能力都手动启动" text="保存连接不会调用模型，也不会触发策略或回测。" /><Guardrail icon={<ShieldCheck />} title="AI 没有执行权限" text="模型只能生成受限草案或解释已存在的数据。" /></section>
    </div>
  )
}

function IntegrationCard({ integration, active, onOpen, onClose }: { integration: (typeof integrations)[number]; active: boolean; onOpen: () => void; onClose: () => void }) {
  const Icon = integration.icon
  const configurable = integration.id === 'ai' || integration.id === 'moomoo'
  return <article className={`min-w-0 overflow-hidden rounded-[1.35rem] border bg-white p-5 transition-colors ${active ? 'border-[#2d6a57]' : 'border-slate-200 hover:border-slate-300'} ${configurable && active ? 'md:col-span-2' : ''}`}><div className="flex items-start justify-between gap-4"><span className="grid size-10 place-items-center rounded-xl bg-[#e5eff4] text-[#294f60]"><Icon className="size-5" /></span><span className={`rounded-full px-2.5 py-1 text-xs font-medium ${integration.id === 'docker' ? 'bg-[#e6f2eb] text-[#2d6a57]' : 'bg-slate-100 text-slate-500'}`}>{integration.id === 'docker' ? '本机运行' : configurable ? '可在页面连接' : '随行情连接'}</span></div><h2 className="mt-5 text-lg font-semibold tracking-[-0.025em] text-[#102028]">{integration.title}</h2><p className="mt-2 min-h-12 text-sm leading-6 text-slate-600">{integration.description}</p>{active ? integration.id === 'ai' ? <AiConnectionForm onClose={onClose} /> : integration.id === 'moomoo' ? <OpenDConnectionForm onClose={onClose} /> : <ConnectionBoundary id={integration.id} onClose={onClose} /> : <button type="button" onClick={onOpen} className="mt-5 inline-flex items-center gap-1 text-sm font-medium text-[#2d6a57]">{integration.id === 'ai' ? '输入 API 配置' : integration.id === 'moomoo' ? '输入 OpenD 配置' : '查看连接边界'} <ChevronRight className="size-3.5" /></button>}</article>
}

function OpenDConnectionForm({ onClose }: { onClose: () => void }) {
  const runtime = useRuntimeStatus()
  const configure = useConfigureSessionOpenD()
  const clear = useClearSessionOpenD()
  const [host, setHost] = useState('127.0.0.1')
  const [port, setPort] = useState('11111')
  const configured = runtime.data?.market_data === 'configured' && runtime.data?.historical_prices === 'configured'

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    try {
      await configure.mutateAsync({ host: host.trim(), port: Number(port) })
    } catch {
      // A bounded inline error explains invalid or unavailable local configuration.
    }
  }

  return <div className="mt-5 rounded-2xl border border-[#d6e3dd] bg-[#f7faf8] p-4 sm:p-5">
    <div className="flex flex-wrap items-start justify-between gap-3"><div><p className="text-sm font-semibold text-[#102028]">本次后端运行的只读行情连接</p><p className="mt-1 text-xs leading-5 text-slate-500">仅接受本机回环地址。保存时不会读取账户，也不会测试下单；第一次行情或回测请求才会实际连接 OpenD。</p></div>{configured ? <span className="rounded-full bg-[#e5f1ea] px-3 py-1.5 text-xs font-semibold text-[#2d6a57]">只读行情已配置</span> : null}</div>
    <form className="mt-5 grid gap-4 sm:grid-cols-[minmax(0,1fr)_10rem]" onSubmit={(event) => void submit(event)}>
      <label className="grid gap-1.5 text-sm font-medium text-[#102028]">本机地址<Input aria-label="OpenD 本机地址" value={host} onChange={(event) => setHost(event.target.value)} placeholder="127.0.0.1" className="h-11 bg-white" /></label>
      <label className="grid gap-1.5 text-sm font-medium text-[#102028]">端口<Input aria-label="OpenD 端口" type="number" min={1} max={65535} value={port} onChange={(event) => setPort(event.target.value)} className="h-11 bg-white" /></label>
      {configure.isError ? <p role="alert" className="text-sm text-[#8a5d27] sm:col-span-2">配置未保存。地址必须是本机回环 IP，端口必须有效。</p> : null}
      {configure.isSuccess ? <p role="status" className="text-sm text-[#2d6a57] sm:col-span-2">只读行情配置已保存到当前后端进程；没有开启模拟券商或订单权限。</p> : null}
      <div className="flex flex-wrap gap-2 sm:col-span-2"><Button type="submit" disabled={!host.trim() || !port || configure.isPending} className="rounded-full bg-[#102830] px-5 text-white">{configure.isPending ? <><Loader2 className="animate-spin" />正在保存…</> : '保存只读连接'}</Button>{configured ? <Button type="button" variant="outline" disabled={clear.isPending} onClick={() => clear.mutate()} className="rounded-full border-[#d5c5ba] text-[#735342]"><Trash2 />清除本次连接</Button> : null}<Button type="button" variant="ghost" onClick={onClose} className="rounded-full">收起</Button></div>
    </form>
  </div>
}

function AiConnectionForm({ onClose }: { onClose: () => void }) {
  const providers = useAiProviders()
  const configure = useConfigureSessionAiProvider()
  const clear = useClearSessionAiProvider()
  const probe = useTestSessionAiProvider()
  const sessionProvider = (providers.data?.providers ?? []).find((provider) => provider.id.startsWith('session-'))
  const [provider, setProvider] = useState<ConfigureSessionAiProviderRequest['provider']>('qwen_cloud')
  const [model, setModel] = useState(providerDefaults.qwen_cloud)
  const [apiKey, setApiKey] = useState('')

  const submit = async (event: FormEvent) => {
    event.preventDefault()
    if (!apiKey.trim() || !model.trim()) return
    try {
      probe.reset()
      await configure.mutateAsync({ provider, model: model.trim(), api_key: apiKey.trim() })
      setApiKey('')
    } catch {
      // The mutation exposes a bounded inline error; the key remains available for correction.
    }
  }

  const chooseProvider = (next: ConfigureSessionAiProviderRequest['provider']) => {
    probe.reset()
    setProvider(next)
    setModel(providerDefaults[next])
  }

  const probeResult = probe.data ? probeCopy[probe.data.status] : null
  const badge = probe.data?.status === 'available'
    ? { text: '连接已验证', className: 'bg-[#e5f1ea] text-[#2d6a57]' }
    : probe.data
      ? { text: '验证未通过', className: 'bg-[#f7ecd5] text-[#7a5520]' }
      : { text: '凭据已保存 · 尚未验证', className: 'bg-[#e5f1ea] text-[#2d6a57]' }

  return <div className="mt-5 min-w-0 rounded-2xl border border-[#d6e3dd] bg-[#f7faf8] p-4 sm:p-5">
    <div className="flex flex-wrap items-start justify-between gap-3"><div className="min-w-0"><p className="text-sm font-semibold text-[#102028]">本次后端运行的 AI 连接</p><p className="mt-1 text-xs leading-5 text-slate-500">保存不会调用模型；“验证 AI 可用性”会发起一条最小请求，不保存提示词或响应。QwenCloud 首次响应可能接近一分钟。</p></div>{sessionProvider ? <span className={`max-w-full break-words rounded-full px-3 py-1.5 text-xs font-semibold ${badge.className}`}>{badge.text}</span> : null}</div>
    <form className="mt-5 grid gap-4 sm:grid-cols-2" onSubmit={(event) => void submit(event)}>
      <label className="grid gap-1.5 text-sm font-medium text-[#102028]">服务商<select aria-label="AI 服务商" value={provider} onChange={(event) => chooseProvider(event.target.value as ConfigureSessionAiProviderRequest['provider'])} className="h-11 rounded-xl border border-[#c8d8d1] bg-white px-3 text-sm outline-none focus:border-[#2d6a57] focus:ring-3 focus:ring-[#b8d5c6]/40"><option value="qwen_cloud">QwenCloud · sk-ws-</option><option value="qwen">阿里云百炼 / DashScope</option><option value="gpt">GPT / OpenAI</option><option value="claude">Claude</option><option value="deepseek">DeepSeek</option></select></label>
      <label className="grid gap-1.5 text-sm font-medium text-[#102028]">模型名称<Input aria-label="AI 模型名称" value={model} maxLength={120} onChange={(event) => setModel(event.target.value)} className="h-11 bg-white" /></label>
      <label className="grid gap-1.5 text-sm font-medium text-[#102028] sm:col-span-2">API Key<Input aria-label="AI API Key" type="password" autoComplete="new-password" value={apiKey} maxLength={512} onChange={(event) => setApiKey(event.target.value)} placeholder="只发送到当前本机后端，不会回显" className="h-11 bg-white" /></label>
      {configure.isError ? <p role="alert" className="text-sm text-[#8a5d27] sm:col-span-2">连接配置未保存。请检查服务商、模型名称和 API Key。</p> : null}
      {configure.isSuccess && !probe.data ? <p role="status" className="text-sm text-[#2d6a57] sm:col-span-2">凭据已保存到当前后端进程，但尚未向供应商验证。现在可手动执行一次最小验证。</p> : null}
      {probeResult ? <div role="status" aria-live="polite" className={`rounded-xl border px-4 py-3 sm:col-span-2 ${probeResult.className}`}><p className="text-sm font-semibold">{probeResult.title}</p><p className="mt-1 text-xs leading-5 opacity-90">{probeResult.detail}</p></div> : null}
      {probe.isError ? <p role="alert" className="text-sm text-[#8a5d27] sm:col-span-2">验证请求没有到达当前后端。请确认 Rust 服务仍在运行后重试。</p> : null}
      <div className="flex flex-wrap gap-2 sm:col-span-2"><Button type="submit" disabled={!apiKey.trim() || !model.trim() || configure.isPending} className="rounded-full bg-[#102830] px-5 text-white">{configure.isPending ? <><Loader2 className="animate-spin" />正在保存…</> : '保存本次连接'}</Button>{sessionProvider ? <Button type="button" variant="outline" disabled={probe.isPending} onClick={() => probe.mutate()} className="rounded-full border-[#9fc8b5] text-[#245b49]">{probe.isPending ? <><Loader2 className="animate-spin" />正在验证…</> : <><CheckCircle2 />验证 AI 可用性</>}</Button> : null}{sessionProvider ? <Button type="button" variant="outline" disabled={clear.isPending} onClick={() => { probe.reset(); clear.mutate() }} className="rounded-full border-[#d5c5ba] text-[#735342]"><Trash2 />清除连接</Button> : null}<Button type="button" variant="ghost" onClick={onClose} className="rounded-full">收起</Button></div>
    </form>
  </div>
}

function ConnectionBoundary({ id, onClose }: { id: Exclude<IntegrationId, 'ai' | 'moomoo'>; onClose: () => void }) {
  const copy = id === 'docker'
    ? ['SQLite 数据与 Docker 挂载在启动前确定。', '运行中的页面不会改写本机路径或容器权限。']
    : ['市场数据连接与 Moomoo / OpenD 共用同一个只读配置。', '回测页会明确显示实际提供方、有效时间和校验值。']
  return <div aria-live="polite" className="mt-5 rounded-xl bg-[#f6f8f6] p-4"><p className="text-sm font-medium text-[#102028]">连接边界</p><ul className="mt-2 list-disc space-y-1 pl-5 text-xs leading-6 text-slate-600">{copy.map((line) => <li key={line}>{line}</li>)}</ul><button type="button" onClick={onClose} className="mt-3 text-sm font-medium text-[#2d6a57]">收起</button></div>
}

function Guardrail({ icon, title, text }: { icon: React.ReactNode; title: string; text: string }) { return <div className="rounded-[1.2rem] bg-[#eef4f1] p-5 text-sm"><span className="text-[#2d6a57]">{icon}</span><h3 className="mt-3 font-semibold text-[#102028]">{title}</h3><p className="mt-1.5 leading-6 text-slate-600">{text}</p></div> }
