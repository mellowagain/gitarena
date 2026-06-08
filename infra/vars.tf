// Local vars
variable "frontend_domain" {
  type    = string
  default = "git.mari.zip"
}

variable "backend_domain" {
  type    = string
  default = "api.git.mari.zip"
}

variable "local_ip_block" {
  type      = string
  sensitive = true
}

// Tencent Cloud
variable "tencent_cloud_secret_id" {
  type      = string
  sensitive = true
}

variable "tencent_cloud_secret_key" {
  type      = string
  sensitive = true
}

// Cloudflare
variable "cloudflare_api_token" {
  type      = string
  sensitive = true
}

variable "cloudflare_zone_id" {
  type = string
}

variable "cloudflare_account_id" {
  type = string
}

// Aiven
variable "aiven_api_token" {
  type      = string
  sensitive = true
}

// Vercel
variable "vercel_api_token" {
  type      = string
  sensitive = true
}

variable "vercel_team" {
  type = string
}

// Resend
variable "resend_api_key" {
  type      = string
  sensitive = true
}

// NewRelic
variable "newrelic_account_id" {
  type = string
}

variable "newrelic_api_token" {
  type      = string
  sensitive = true
}
