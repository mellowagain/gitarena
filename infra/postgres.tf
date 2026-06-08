resource "aiven_pg" "main" {
  project      = "gitarena"
  service_name = "gitarena"
  cloud_name   = "do-ams"
  plan         = "free-1-1gb"

  termination_protection = true

  pg_user_config {
    pg_version = "17"

    ip_filter_string = [
      "162.62.58.144/32", // todo: change to tencent ip once its hooked up to tf
      var.local_ip_block,
    ]
  }
}
