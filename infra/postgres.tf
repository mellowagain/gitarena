resource "aiven_pg" "main" {
  project      = "gitarena"
  service_name = "gitarena"
  cloud_name   = "do-ams"
  plan         = "free-1-1gb"

  pg_user_config {
    pg_version = "17"
  }
}
