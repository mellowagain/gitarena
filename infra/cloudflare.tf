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
