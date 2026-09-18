import { Outlet } from 'react-router'

import { AppHeader } from './app-header'
import { AppSidebar } from './app-sidebar'

export function AppLayout() {
  return (
    <div className="min-h-svh bg-[#f6f8f6] text-[#102028]">
      <AppHeader />
      <div className="mx-auto flex min-h-[calc(100svh-4.5rem)] w-full max-w-[1440px]">
        <AppSidebar />
        <main className="min-w-0 flex-1 overflow-x-hidden">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
