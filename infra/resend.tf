resource "resend_domain" "main" {
  name   = "send.${var.frontend_domain}"
  region = "eu-west-1"
}
