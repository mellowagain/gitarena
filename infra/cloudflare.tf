resource "cloudflare_r2_bucket" "artifacts" {
  account_id = var.cloudflare_account_id
  name       = "gitarena"
  location   = "WEUR"
}

resource "cloudflare_r2_custom_domain" "artifacts" {
  account_id  = var.cloudflare_account_id
  bucket_name = cloudflare_r2_bucket.artifacts.name
  domain      = "objects.${var.frontend_domain}"
  zone_id     = var.cloudflare_zone_id
  enabled     = true
}

resource "cloudflare_r2_bucket_cors" "artifacts" {
  account_id  = var.cloudflare_account_id
  bucket_name = cloudflare_r2_bucket.artifacts.name

  rules = [{
    allowed = {
      origins = ["https://${var.frontend_domain}", "https://${var.backend_domain}"]
      methods = ["GET", "PUT"]
      headers = ["*"]
    }
    expose = {
      headers = ["ETag"]
    }
    max_age_seconds = 3600
  }]
}
