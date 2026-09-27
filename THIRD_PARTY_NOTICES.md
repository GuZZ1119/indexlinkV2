# Third-party notices and research references

IndexLink is MIT-licensed, but its dependencies, external services, research sources, and data sources retain their own licenses and terms. This file records the main references that materially influenced V2.1. A citation is not a license grant: exact data-file status and release decisions are tracked in [`DATA_LICENSES.md`](./DATA_LICENSES.md), while new strategy contributions must follow [`docs/contributing/strategy-source-policy.md`](./docs/contributing/strategy-source-policy.md).

Unless a row explicitly says that code was adapted, the projects and papers below are **references only**. The current Formula runtime is independently implemented in this repository; an upstream software license must not be misread as a license for a paper, dataset, trading result, or index value.

## Architecture and engineering references

| Reference | License / terms | How IndexLink uses it |
| --- | --- | --- |
| [Alistair Cockburn — Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture) | Article copyright remains with the author | Architectural reference for ports, adapters, pure domain logic, and replaceable SQLite/OpenD/AI/broker implementations |
| [QuantConnect LEAN](https://github.com/QuantConnect/Lean) | Apache-2.0 | Cross-check for backtest boundaries, portfolio accounting concepts, and research vocabulary; no LEAN source code is copied into IndexLink |
| [TA-Lib](https://ta-lib.github.io/) | BSD | Indicator naming and semantic reference for SMA, EMA, RSI, rate of change, and related technical concepts; formulas are independently implemented and tested in Rust |

## Strategy and research references

| Reference | Scope in IndexLink |
| --- | --- |
| [Meb Faber — A Quantitative Approach to Tactical Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=962461) | Long-horizon moving-average trend protection and causal evaluation concepts |
| [QuantConnect LEAN examples and indicators](https://github.com/QuantConnect/Lean) | Strategy-family taxonomy and backtest cross-checks |
| [TA-Lib indicator documentation](https://ta-lib.github.io/) | Standard indicator terminology and expected behavior |
| [Awesome Systematic Trading research index](https://github.com/paperswithbacktest/awesome-systematic-trading) | Discovery index for the time-series momentum research family; no repository code is copied |

The 100 official Formula presets are independent combinations of IndexLink's bounded rules and parameters. A citation is research provenance, not an assertion that the cited author endorses a preset or that IndexLink reproduces the cited strategy exactly. No cited strategy is marketed as profitable. If a future contribution copies or adapts third-party code, it must be reclassified as a code adaptation and carry its fixed upstream revision, license, copyright, notices, and modification record.

## Web UI and visualization

| Project | License | Use |
| --- | --- | --- |
| [shadcn/ui](https://github.com/shadcn-ui/ui) | MIT | Copy-and-adapt component patterns and a small local Tailwind data-state variant layer; the CLI package is not shipped as a production dependency |
| [Radix UI](https://github.com/radix-ui/primitives) | MIT | Accessible headless primitives |
| [Lucide](https://github.com/lucide-icons/lucide) | ISC | Interface icons |
| [TanStack Query](https://github.com/TanStack/query) | MIT | Server-state queries, mutations, cache invalidation, and loading/error state |
| [Recharts](https://recharts.github.io/) | MIT | Lightweight charts |
| [Apache ECharts](https://echarts.apache.org/) | Apache-2.0 | Zoomable normalized wealth and trade-marker charts |
| [Tailwind CSS](https://github.com/tailwindlabs/tailwindcss) | MIT | Styling system |

See `apps/web/package.json` and `apps/web/pnpm-lock.yaml` for exact versions and the complete transitive dependency graph.

## Rust ecosystem

IndexLink uses Axum, Tokio, SQLx, Reqwest, rustls, Serde, quick-xml, and other Rust crates under their respective published licenses. `Cargo.toml` and `Cargo.lock` are the authoritative version records. `cargo audit` is used for security advisories; it is not a license scanner.

## External services and data

| Source | Terms / documentation | Use and boundary |
| --- | --- | --- |
| Futu / Moomoo OpenD | [Official OpenAPI introduction](https://openapi.futunn.com/futu-api-doc/en/intro/intro.html) | User-operated local gateway for read-only daily history and optional paper-account experiments; availability and entitlements belong to the user's account |
| FRED | [API terms](https://fred.stlouisfed.org/docs/api/terms_of_use.html) and [general terms](https://fred.stlouisfed.org/legal/terms/) | Historical research source. FRED states that some series have third-party rights and that access does not override them. Redistribution of the committed snapshots is not confirmed; this product is not endorsed or certified by the Federal Reserve Bank of St. Louis |
| Cboe | [Data policies](https://datashop.cboe.com/data-policies) | VIX research source. No written grant covering redistribution in this repository has been recorded; the committed snapshot is therefore release-blocked pending permission or replacement |
| Multpl | [Website](https://www.multpl.com/shiller-pe/table/by-month) | Historical Shiller P/E provenance. The raw fetched HTML is intentionally excluded. No explicit license covering redistribution of the sanitized monthly extract has been confirmed |

Generated manifests under `crates/strategy-evaluation/data/generated/` record source URLs, date ranges, assumptions, and checksums. Inclusion of source metadata does not grant redistribution rights or imply endorsement. Derived fixtures and reports can retain upstream data restrictions even though the generator is MIT-licensed. The current file-by-file decision is intentionally fail-closed in `DATA_LICENSES.md`; unresolved snapshots should not be represented as MIT data or included in a formal release until permission or a compatible replacement is documented.

## AI providers

QwenCloud, Alibaba Cloud Model Studio / DashScope, OpenAI, Anthropic, and DeepSeek are optional user-selected services. IndexLink does not bundle credentials or proxy access. Users are responsible for provider terms, costs, regional availability, and data-handling policies. Provider names are used only to identify compatible API protocols.
