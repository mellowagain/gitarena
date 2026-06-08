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

provider "resend" {
  api_key = var.resend_api_key
}

provider "newrelic" {
  account_id = var.newrelic_account_id
  api_key    = var.newrelic_api_token
  region     = "EU"
}
