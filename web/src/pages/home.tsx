import { ArrowRight, Boxes, Code2, Server } from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"

const foundations = [
  {
    icon: Code2,
    title: "React + Vite",
    description: "Fast local development with a type-safe React foundation.",
  },
  {
    icon: Boxes,
    title: "shadcn/ui",
    description: "Composable interface primitives that stay in your codebase.",
  },
  {
    icon: Server,
    title: "Go API",
    description: "A typed API layer is ready for the Lux server endpoints.",
  },
]

export function HomePage() {
  return (
    <div className="space-y-10">
      <section className="max-w-3xl space-y-5">
        <p className="text-sm font-medium text-primary">Lux platform</p>
        <h1 className="text-4xl font-semibold tracking-tight sm:text-6xl">
          A bright foundation for building with clarity.
        </h1>
        <p className="max-w-2xl text-lg text-muted-foreground">
          The Lux frontend is ready for pages, reusable components, and a typed
          connection to the Go server.
        </p>
        <Button>
          Start building <ArrowRight className="size-4" />
        </Button>
      </section>

      <section className="grid gap-4 md:grid-cols-3" aria-label="Frontend foundation">
        {foundations.map((foundation) => {
          const Icon = foundation.icon

          return (
            <Card key={foundation.title}>
              <CardHeader>
                <div className="mb-2 flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Icon className="size-5" />
                </div>
                <CardTitle>{foundation.title}</CardTitle>
                <CardDescription>{foundation.description}</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="h-1 w-16 rounded-full bg-primary/20" />
              </CardContent>
            </Card>
          )
        })}
      </section>
    </div>
  )
}
