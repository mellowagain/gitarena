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
    resend = {
      source  = "registry.terraform.io/jhoward321/resend"
      version = "0.1.3"
    }
    newrelic = {
      source  = "newrelic/newrelic"
      version = "3.81.0"
    }
  }
}


