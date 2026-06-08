resource "tencentcloud_lighthouse_instance" "main" {
  instance_name        = "Ubuntu-YUz3"
  zone                 = "eu-frankfurt-1"
  bundle_id            = "bundle_starter_nmc_lin_med2_01"
  blueprint_id         = "lhbp-b46k6f98"
  renew_flag           = "NOTIFY_AND_MANUAL_RENEW"
  firewall_template_id = tencentcloud_lighthouse_firewall_template.main_firewall.id
}

resource "tencentcloud_lighthouse_firewall_template" "main_firewall" {
  template_name = "gitarena"

  template_rules {
    protocol                  = "TCP"
    port                      = "80,443"
    action                    = "ACCEPT"
    firewall_rule_description = "caddy"
  }

  template_rules {
    protocol                  = "TCP"
    port                      = "22,2222"
    action                    = "ACCEPT"
    firewall_rule_description = "ssh"
  }

  template_rules {
    protocol                  = "ICMP"
    port                      = "ALL"
    action                    = "ACCEPT"
    firewall_rule_description = "ping"
  }
}
