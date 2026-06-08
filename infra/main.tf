terraform {
  required_version = ">= 1.11"

  required_providers {
    tencentcloud = {
      source  = "tencentcloudstack/tencentcloud"
      version = "1.82.74"
    }
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "5.19.0-beta.1"
    }
    aiven = {
      source  = "aiven/aiven"
      version = "4.52.0"
    }
    vercel = {
      source  = "vercel/vercel"
      version = "4.6.1"
    }
    newrelic = {
      source  = "newrelic/newrelic"
      version = "3.81.0"
    }
  }
}

provider "tencentcloud" {
  secret_id  = var.tencent_cloud_secret_id
  secret_key = var.tencent_cloud_secret_key
  region     = "eu-frankfurt"
}

provider "cloudflare" {
  api_token = var.cloudflare_api_token
}

provider "aiven" {
  api_token = var.aiven_api_token
}

provider "vercel" {
  api_token = var.vercel_api_token
  team      = var.vercel_team
}

provider "newrelic" {
  account_id = var.newrelic_account_id
  api_key    = var.newrelic_api_token
  region     = "EU"
}
