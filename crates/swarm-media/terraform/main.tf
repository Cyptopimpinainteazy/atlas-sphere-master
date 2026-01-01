# ============================================================================
# X3 ATLAS SPHERE - SWARM MEDIA TERRAFORM CONFIGURATION
# Infrastructure-as-Code for GKE deployment with monitoring and logging
# ============================================================================

terraform {
  required_version = ">= 1.0"
  
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 5.0"
    }
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
    helm = {
      source  = "hashicorp/helm"
      version = "~> 2.11"
    }
  }
  
  backend "gcs" {
    bucket = "x3-atlas-sphere-terraform-state"
    prefix = "swarm-media"
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

provider "kubernetes" {
  host                   = "https://${google_container_cluster.primary.endpoint}"
  token                  = data.google_client_config.default.access_token
  cluster_ca_certificate = base64decode(google_container_cluster.primary.master_auth[0].cluster_ca_certificate)
}

provider "helm" {
  kubernetes {
    host                   = "https://${google_container_cluster.primary.endpoint}"
    token                  = data.google_client_config.default.access_token
    cluster_ca_certificate = base64decode(google_container_cluster.primary.master_auth[0].cluster_ca_certificate)
  }
}

data "google_client_config" "default" {}

# ============================================================================
# VARIABLES
# ============================================================================

variable "project_id" {
  description = "GCP Project ID"
  type        = string
}

variable "region" {
  description = "GCP Region"
  type        = string
  default     = "us-central1"
}

variable "cluster_name" {
  description = "GKE Cluster Name"
  type        = string
  default     = "swarm-media-cluster"
}

variable "network_name" {
  description = "VPC Network Name"
  type        = string
  default     = "swarm-media-network"
}

variable "node_count" {
  description = "Initial node count"
  type        = number
  default     = 3
}

variable "machine_type" {
  description = "GKE Node Machine Type"
  type        = string
  default     = "n2-standard-4"
}

variable "min_node_count" {
  description = "Minimum node count for autoscaling"
  type        = number
  default     = 3
}

variable "max_node_count" {
  description = "Maximum node count for autoscaling"
  type        = number
  default     = 10
}

variable "disk_size_gb" {
  description = "Node disk size in GB"
  type        = number
  default     = 100
}

variable "enable_monitoring" {
  description = "Enable Google Cloud Monitoring"
  type        = bool
  default     = true
}

variable "enable_logging" {
  description = "Enable Google Cloud Logging"
  type        = bool
  default     = true
}

variable "environment" {
  description = "Environment (dev, staging, production)"
  type        = string
  default     = "production"
  
  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be dev, staging, or production."
  }
}

# ============================================================================
# NETWORKING
# ============================================================================

resource "google_compute_network" "primary" {
  name                    = var.network_name
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "primary" {
  name          = "${var.network_name}-subnet"
  ip_cidr_range = "10.0.0.0/24"
  region        = var.region
  network       = google_compute_network.primary.id
  
  log_config {
    aggregation_interval = "INTERVAL_5_SEC"
    flow_logs_enabled    = true
    metadata             = "INCLUDE_ALL_METADATA"
  }
}

resource "google_compute_firewall" "allow_internal" {
  name    = "${var.network_name}-allow-internal"
  network = google_compute_network.primary.name
  
  allow {
    protocol = "tcp"
    ports    = ["0-65535"]
  }
  
  allow {
    protocol = "udp"
    ports    = ["0-65535"]
  }
  
  source_ranges = ["10.0.0.0/24"]
}

# ============================================================================
# GKE CLUSTER
# ============================================================================

resource "google_container_cluster" "primary" {
  name     = var.cluster_name
  location = var.region
  
  # Recommended settings for production
  initial_node_count       = var.node_count
  remove_default_node_pool = true
  network                  = google_compute_network.primary.name
  subnetwork               = google_compute_subnetwork.primary.name
  
  # Kubernetes version
  min_master_version = "1.27"
  
  # Enable features
  enable_shielded_nodes = true
  enable_network_policy = true
  
  # IP allocation
  cluster_secondary_range_name = ""
  
  # Logging and monitoring
  logging_service    = var.enable_logging ? "logging.googleapis.com/kubernetes" : "none"
  monitoring_service = var.enable_monitoring ? "monitoring.googleapis.com/kubernetes" : "none"
  
  # Workload Identity
  workload_identity_config {
    workload_pool = "${var.project_id}.svc.id.goog"
  }
  
  # Network policy
  network_policy {
    enabled = true
  }
  
  # Security
  master_auth {
    client_certificate_config {
      issue_client_certificate = false
    }
  }
  
  # Addons
  addons_config {
    http_load_balancing {
      disabled = false
    }
    horizontal_pod_autoscaling {
      disabled = false
    }
    network_policy_config {
      disabled = false
    }
  }
  
  # Maintenance window
  maintenance_policy {
    daily_maintenance_window {
      start_time = "03:00"
    }
  }
  
  labels = {
    environment = var.environment
    app         = "swarm-media"
  }
}

# ============================================================================
# NODE POOL
# ============================================================================

resource "google_container_node_pool" "primary_nodes" {
  name       = "primary-node-pool"
  location   = var.region
  cluster    = google_container_cluster.primary.name
  node_count = var.node_count
  
  autoscaling {
    min_node_count = var.min_node_count
    max_node_count = var.max_node_count
  }
  
  management {
    auto_repair  = true
    auto_upgrade = true
  }
  
  node_config {
    preemptible  = var.environment != "production"
    machine_type = var.machine_type
    disk_size_gb = var.disk_size_gb
    
    oauth_scopes = [
      "https://www.googleapis.com/auth/cloud-platform",
    ]
    
    workload_metadata_config {
      mode = "GKE_METADATA"
    }
    
    shielded_instance_config {
      enable_secure_boot          = true
      enable_integrity_monitoring = true
    }
    
    labels = {
      environment = var.environment
      pool        = "primary"
    }
    
    tags = ["swarm-media", var.environment]
  }
}

# ============================================================================
# SERVICE ACCOUNTS
# ============================================================================

resource "google_service_account" "swarm_media" {
  account_id   = "swarm-media"
  display_name = "Swarm Media Service Account"
}

resource "google_workload_identity_binding" "swarm_media" {
  service_account_id = google_service_account.swarm_media.name
  location           = var.region
  
  attribute {
    key   = "kubernetes.io/namespace"
    value = "swarm-media"
  }
  
  attribute {
    key   = "kubernetes.io/service_account"
    value = "swarm-media"
  }
}

# ============================================================================
# CLOUD STORAGE (for state/backups)
# ============================================================================

resource "google_storage_bucket" "swarm_media" {
  name          = "${var.project_id}-swarm-media-data"
  location      = var.region
  force_destroy = false
  
  uniform_bucket_level_access = true
  
  versioning {
    enabled = true
  }
  
  lifecycle_rule {
    condition {
      num_newer_versions = 10
    }
    action {
      type = "Delete"
    }
  }
  
  labels = {
    environment = var.environment
    app         = "swarm-media"
  }
}

# ============================================================================
# CLOUD SQL (PostgreSQL for metrics/logs)
# ============================================================================

resource "google_sql_database_instance" "swarm_media" {
  name             = "swarm-media-db"
  database_version = "POSTGRES_15"
  region           = var.region
  
  settings {
    tier              = var.environment == "production" ? "db-custom-4-16384" : "db-f1-micro"
    availability_type = var.environment == "production" ? "REGIONAL" : "ZONAL"
    
    backup_configuration {
      enabled            = true
      start_time         = "02:00"
      transaction_log_retention_days = 7
      backup_retention_settings {
        retained_backups = 30
        retention_unit   = "COUNT"
      }
    }
    
    database_flags {
      name  = "max_connections"
      value = var.environment == "production" ? "500" : "100"
    }
    
    database_flags {
      name  = "log_statement"
      value = "ddl"
    }
  }
  
  deletion_protection = var.environment == "production"
}

resource "google_sql_database" "swarm_media" {
  name     = "swarm_media_db"
  instance = google_sql_database_instance.swarm_media.name
}

# ============================================================================
# MONITORING ALERT POLICY
# ============================================================================

resource "google_monitoring_alert_policy" "cluster_health" {
  count = var.enable_monitoring ? 1 : 0
  
  display_name = "Swarm Media - Cluster Health"
  combiner     = "OR"
  
  conditions {
    display_name = "High CPU Usage"
    
    condition_threshold {
      filter          = "resource.type = \"k8s_cluster\" AND metric.type = \"compute.googleapis.com/instance/cpu/utilization\""
      duration        = "300s"
      comparison      = "COMPARISON_GT"
      threshold_value = 0.8
      
      aggregations {
        alignment_period  = "60s"
        per_series_aligner = "ALIGN_MEAN"
      }
    }
  }
  
  conditions {
    display_name = "Node Pool Unhealthy"
    
    condition_threshold {
      filter          = "resource.type = \"k8s_nodepool\" AND metric.type = \"kubernetes.io/node/memory/allocatable_utilization\""
      duration        = "300s"
      comparison      = "COMPARISON_GT"
      threshold_value = 0.9
      
      aggregations {
        alignment_period  = "60s"
        per_series_aligner = "ALIGN_MEAN"
      }
    }
  }
  
  notification_channels = var.enable_monitoring ? [google_monitoring_notification_channel.email[0].name] : []
}

resource "google_monitoring_notification_channel" "email" {
  count           = var.enable_monitoring ? 1 : 0
  display_name    = "Swarm Media Alerts"
  type            = "email"
  enabled         = true
  labels = {
    email_address = "alerts@example.com"
  }
}

# ============================================================================
# OUTPUTS
# ============================================================================

output "kubernetes_cluster_name" {
  value       = google_container_cluster.primary.name
  description = "GKE Cluster Name"
}

output "kubernetes_cluster_host" {
  value       = google_container_cluster.primary.endpoint
  description = "GKE Cluster Host"
  sensitive   = true
}

output "region" {
  value       = var.region
  description = "GCP Region"
}

output "project_id" {
  value       = var.project_id
  description = "GCP Project ID"
}

output "storage_bucket" {
  value       = google_storage_bucket.swarm_media.name
  description = "Cloud Storage Bucket Name"
}

output "database_instance" {
  value       = google_sql_database_instance.swarm_media.connection_name
  description = "Cloud SQL Instance Connection Name"
}
