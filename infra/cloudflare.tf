resource "cloudflare_dns_record" "backend" {
  zone_id = var.cloudflare_zone_id
  name    = var.backend_domain
  type    = "A"
  content = tencentcloud_lighthouse_instance.main.public_addresses[0]
  proxied = false
  ttl     = 1
}

resource "cloudflare_dns_record" "frontend" {
  zone_id = var.cloudflare_zone_id
  name    = var.frontend_domain
  type    = "CNAME"
  content = data.vercel_domain_config.frontend.recommended_cname
  proxied = false
  ttl     = 600
}

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
