import { apiRequest } from "./client"

export type Example = {
  created_at: string
  id: string
  name: string
}

export type CreateExampleRequest = {
  name: string
}

export function createExample(request: CreateExampleRequest) {
  return apiRequest<Example>("/example", {
    method: "POST",
    body: JSON.stringify(request),
  })
}
