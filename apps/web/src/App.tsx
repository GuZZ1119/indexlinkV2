import { lazy, Suspense } from 'react'
import type { ComponentType, LazyExoticComponent } from 'react'
import { createBrowserRouter, Navigate, RouterProvider } from 'react-router'

import { AppLayout } from '@/components/layout/app-layout'
import RouteErrorPage from '@/pages/route-error'

const PersonalPage = lazy(() => import('@/pages/personal'))
const StrategyCenterPage = lazy(() => import('@/pages/strategy-center'))
const StrategyAnalysisPage = lazy(() => import('@/pages/strategy-analysis'))
const LabPage = lazy(() => import('@/pages/lab'))
const DecisionsPage = lazy(() => import('@/pages/decisions'))
const PlansPage = lazy(() => import('@/pages/plans'))
const StrategiesPage = lazy(() => import('@/pages/strategies'))

function PageFallback() {
  return <div className="p-6 text-sm text-muted-foreground">Loading…</div>
}

function LazyPage({ Page }: { Page: LazyExoticComponent<ComponentType> }) {
  return <Suspense fallback={<PageFallback />}><Page /></Suspense>
}

const router = createBrowserRouter([
  {
    element: <AppLayout />,
    errorElement: <RouteErrorPage />,
    children: [
      { index: true, element: <Navigate to="/personal" replace /> },
      { path: '/personal', element: <LazyPage Page={PersonalPage} /> },
      { path: '/strategy-center', element: <LazyPage Page={StrategyCenterPage} /> },
      { path: '/strategy-analysis', element: <LazyPage Page={StrategyAnalysisPage} /> },
      { path: '/lab', element: <LazyPage Page={LabPage} /> },
      { path: '/decisions/:id?', element: <LazyPage Page={DecisionsPage} /> },
      { path: '/plans/:id?', element: <LazyPage Page={PlansPage} /> },
      { path: '/strategy-studio', element: <LazyPage Page={StrategiesPage} /> },
    ],
  },
])

export default function App() {
  return <RouterProvider router={router} />
}
