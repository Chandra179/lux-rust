import { ArrowUpRight, Hexagon } from "lucide-react"
import { NavLink, Outlet } from "react-router-dom"

import { Button } from "@/components/ui/button"

const navigation = [{ label: "Home", to: "/" }]

export function AppShell() {
  return (
    <div className="min-h-svh bg-muted/30">
      <header className="border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-6">
          <NavLink className="flex items-center gap-2 font-semibold" to="/">
            <span className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
              <Hexagon className="size-4" />
            </span>
            <span>Lux</span>
          </NavLink>

          <nav className="flex items-center gap-1" aria-label="Main navigation">
            {navigation.map((item) => (
              <NavLink
                className={({ isActive }) =>
                  `rounded-md px-3 py-2 text-sm transition-colors ${
                    isActive
                      ? "bg-accent font-medium text-accent-foreground"
                      : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  }`
                }
                key={item.to}
                to={item.to}
              >
                {item.label}
              </NavLink>
            ))}
          </nav>

          <Button asChild size="sm" variant="outline">
            <a href="/swagger/index.html" target="_blank" rel="noreferrer">
              API docs <ArrowUpRight className="size-3.5" />
            </a>
          </Button>
        </div>
      </header>

      <main className="mx-auto w-full max-w-6xl px-6 py-10">
        <Outlet />
      </main>
    </div>
  )
}
