import { apiRequest } from "./client"

export type HealthResponse = {
  status: "ok"
}

export type ReadinessResponse = {
  status: "ready"
}

export function getHealth() {
  return apiRequest<HealthResponse>("/health")
}

export function getReadiness() {
  return apiRequest<ReadinessResponse>("/ready")
}
