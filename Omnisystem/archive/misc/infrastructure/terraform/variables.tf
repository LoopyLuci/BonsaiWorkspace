# Omnisystem Phase 3: Terraform Variables

variable "gcp_project_id" {
  description = "GCP Project ID"
  type        = string
}

variable "gcp_region" {
  description = "GCP Region"
  type        = string
  default     = "us-central1"
}

variable "initial_node_count" {
  description = "Initial number of nodes in the cluster"
  type        = number
  default     = 3
}

variable "min_nodes" {
  description = "Minimum number of nodes in the cluster"
  type        = number
  default     = 3
}

variable "max_nodes" {
  description = "Maximum number of nodes in the cluster"
  type        = number
  default     = 100
}

variable "machine_type" {
  description = "Machine type for nodes"
  type        = string
  default     = "n1-standard-4"
}

variable "use_preemptible_nodes" {
  description = "Use preemptible nodes for cost savings"
  type        = bool
  default     = false
}

variable "db_password" {
  description = "PostgreSQL password"
  type        = string
  sensitive   = true
}

variable "redis_memory_gb" {
  description = "Redis memory in GB"
  type        = number
  default     = 16
}

variable "admin_ip_ranges" {
  description = "Admin IP ranges for SSH access"
  type        = list(string)
  default     = ["0.0.0.0/0"]  # Restrict in production
}
