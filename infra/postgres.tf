resource "aiven_pg" "main" {
  project      = "gitarena"
  service_name = "gitarena"
  cloud_name   = "do-ams"
  plan         = "free-1-1gb"

  termination_protection = true

  pg_user_config {
    pg_version = "17"

    ip_filter_string = [
      "${tencentcloud_lighthouse_instance.main.public_addresses[0]}/32",
      var.local_ip_block,
    ]
  }
}
