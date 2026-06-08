resource "tencentcloud_lighthouse_instance" "main" {
  instance_name = "Ubuntu-YUz3"
  zone          = "eu-frankfurt-1"
  bundle_id     = "bundle_starter_nmc_lin_med2_01"
  blueprint_id  = "lhbp-b46k6f98"
  renew_flag    = "NOTIFY_AND_MANUAL_RENEW"
}
