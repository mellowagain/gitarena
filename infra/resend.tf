resource "resend_domain" "main" {
  name   = var.frontend_domain
  region = "eu-west-1"
}
