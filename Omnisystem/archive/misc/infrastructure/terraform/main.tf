# Omnisystem Phase 3: Infrastructure-as-Code - Main Kubernetes Cluster
# GKE cluster provisioning with complete networking and security

terraform {
  required_version = ">= 1.0"
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
  }
}

provider "google" {
  project = var.gcp_project_id
  region  = var.gcp_region
}

# GKE Cluster
resource "google_container_cluster" "omnisystem" {
  name     = "omnisystem-cluster"
  location = var.gcp_region

  # Initial node pool - will be replaced by node_pool resource
  remove_default_node_pool = true
  initial_node_count       = 1

  # Networking
  network    = google_compute_network.omnisystem.name
  subnetwork = google_compute_subnetwork.omnisystem.name

  # Security and features
  enable_network_policy = true
  enable_intra_node_visibility = true

  workload_identity_config {
    workload_pool = "${var.gcp_project_id}.svc.id.goog"
  }

  # Maintenance window
  maintenance_policy {
    daily_maintenance_window {
      start_time = "03:00"
    }
  }

  # Cluster labels
  resource_labels = {
    environment = "production"
    application = "omnisystem"
  }
}

# Node pool for Omnisystem crates
resource "google_container_node_pool" "omnisystem_nodes" {
  name       = "omnisystem-nodes"
  cluster    = google_container_cluster.omnisystem.name
  location   = var.gcp_region
  node_count = var.initial_node_count

  autoscaling {
    min_node_count = var.min_nodes
    max_node_count = var.max_nodes
  }

  management {
    auto_repair  = true
    auto_upgrade = true
  }

  node_config {
    preemptible  = var.use_preemptible_nodes
    machine_type = var.machine_type

    disk_size_gb = 100
    disk_type    = "pd-standard"

    oauth_scopes = [
      "https://www.googleapis.com/auth/cloud-platform"
    ]

    workload_metadata_config {
      mode = "GKE_METADATA"
    }

    metadata = {
      disable-legacy-endpoints = "true"
    }

    shielded_instance_config {
      enable_secure_boot          = true
      enable_integrity_monitoring = true
    }

    resource_labels = {
      node_pool = "omnisystem"
    }
  }
}

# VPC Network
resource "google_compute_network" "omnisystem" {
  name                    = "omnisystem-network"
  auto_create_subnetworks = false
  routing_mode            = "REGIONAL"
}

# Subnet
resource "google_compute_subnetwork" "omnisystem" {
  name          = "omnisystem-subnet"
  ip_cidr_range = "10.0.0.0/20"
  region        = var.gcp_region
  network       = google_compute_network.omnisystem.id

  private_ip_google_access = true

  secondary_ip_range {
    range_name    = "pods"
    ip_cidr_range = "10.4.0.0/14"
  }

  secondary_ip_range {
    range_name    = "services"
    ip_cidr_range = "10.8.0.0/20"
  }
}

# Firewall - Allow internal traffic
resource "google_compute_firewall" "allow_internal" {
  name    = "omnisystem-allow-internal"
  network = google_compute_network.omnisystem.name

  allow {
    protocol = "tcp"
    ports    = ["0-65535"]
  }

  allow {
    protocol = "udp"
    ports    = ["0-65535"]
  }

  source_ranges = ["10.0.0.0/20", "10.4.0.0/14", "10.8.0.0/20"]
}

# Firewall - Allow SSH from admin IPs
resource "google_compute_firewall" "allow_ssh" {
  name    = "omnisystem-allow-ssh"
  network = google_compute_network.omnisystem.name

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  source_ranges = var.admin_ip_ranges
}

# Cloud SQL - PostgreSQL
resource "google_sql_database_instance" "postgres" {
  name                = "omnisystem-postgres"
  database_version    = "POSTGRES_15"
  region              = var.gcp_region
  deletion_protection = true

  settings {
    tier              = "db-custom-4-16384"
    availability_type = "REGIONAL"

    backup_configuration {
      enabled                        = true
      point_in_time_recovery_enabled = true
      backup_retention_days          = 30
      start_time                     = "02:00"
    }

    database_flags {
      name  = "max_connections"
      value = "1000"
    }

    database_flags {
      name  = "shared_preload_libraries"
      value = "pg_stat_statements"
    }

    ip_configuration {
      require_ssl    = true
      ipv4_enabled   = true
      ipv6_enabled   = false

      authorized_networks {
        name  = "omnisystem-network"
        value = "10.0.0.0/20"
      }
    }

    backup_configuration {
      binary_log_enabled = false
    }

    insights_config {
      query_insights_enabled  = true
      query_string_length    = 1024
      record_application_tags = true
    }
  }

  deletion_protection = true
}

# PostgreSQL Database
resource "google_sql_database" "omnisystem" {
  name     = "omnisystem"
  instance = google_sql_database_instance.postgres.name
}

# PostgreSQL User
resource "google_sql_user" "omnisystem" {
  name     = "omnisystem"
  instance = google_sql_database_instance.postgres.name
  password = var.db_password
}

# Cloud Memorystore - Redis
resource "google_redis_instance" "cache" {
  name              = "omnisystem-cache"
  tier              = "standard_ha"
  memory_size_gb    = var.redis_memory_gb
  region            = var.gcp_region
  redis_version     = "7.0"

  authorized_network = google_compute_network.omnisystem.id

  transit_encryption_mode = "SERVER_AUTHENTICATION"
  auth_enabled           = true

  maintenance_policy {
    weekly_maintenance_window {
      day = "SUNDAY"
      start_time {
        hours   = 2
        minutes = 0
      }
    }
  }

  labels = {
    environment = "production"
    application = "omnisystem"
  }
}

# Outputs
output "kubernetes_cluster_name" {
  value = google_container_cluster.omnisystem.name
}

output "kubernetes_cluster_location" {
  value = google_container_cluster.omnisystem.location
}

output "postgres_connection_name" {
  value = google_sql_database_instance.postgres.connection_name
}

output "redis_host" {
  value = google_redis_instance.cache.host
}

output "redis_port" {
  value = google_redis_instance.cache.port
}
