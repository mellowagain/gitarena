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
  content = trimsuffix(data.vercel_domain_config.frontend.recommended_cname, ".")
  proxied = false
  ttl     = 600
}

resource "cloudflare_dns_record" "mail" {
  zone_id  = var.cloudflare_zone_id
  name     = resend_domain.main.spf_mx_record.name
  type     = resend_domain.main.spf_mx_record.type
  content  = resend_domain.main.spf_mx_record.value
  priority = resend_domain.main.spf_mx_record.priority
  proxied  = false
  ttl      = try(tonumber(resend_domain.main.spf_mx_record.ttl), 3600)
}

resource "cloudflare_dns_record" "mail_txt" {
  zone_id = var.cloudflare_zone_id
  name    = resend_domain.main.spf_txt_record.name
  type    = resend_domain.main.spf_txt_record.type
  content = resend_domain.main.spf_txt_record.value
  proxied = false
  ttl     = try(tonumber(resend_domain.main.spf_txt_record.ttl), 3600)
}

resource "cloudflare_dns_record" "mail_dkim" {
  zone_id = var.cloudflare_zone_id
  name    = resend_domain.main.dkim_records[0].name
  type    = resend_domain.main.dkim_records[0].type
  content = resend_domain.main.dkim_records[0].value
  proxied = false
  ttl     = try(tonumber(resend_domain.main.dkim_records[0].ttl), 3600)
}
