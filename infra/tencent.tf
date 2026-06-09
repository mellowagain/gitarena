resource "tencentcloud_lighthouse_instance" "main" {
  instance_name        = "Ubuntu-YUz3"
  zone                 = "eu-frankfurt-1"
  bundle_id            = "bundle_starter_nmc_lin_med2_01"
  blueprint_id         = "lhbp-b46k6f98"
  renew_flag           = "NOTIFY_AND_MANUAL_RENEW"
  firewall_template_id = tencentcloud_lighthouse_firewall_template.empty.id

  lifecycle {
    ignore_changes = [firewall_template_id]
  }
}

resource "tencentcloud_lighthouse_firewall_rule" "main_firewall" {
  instance_id = tencentcloud_lighthouse_instance.main.id

  firewall_rules {
    protocol                  = "TCP"
    port                      = "443,80"
    cidr_block                = "0.0.0.0/0"
    action                    = "ACCEPT"
    firewall_rule_description = "caddy"
  }

  firewall_rules {
    protocol                  = "TCP"
    port                      = "22,2222"
    cidr_block                = "0.0.0.0/0"
    action                    = "ACCEPT"
    firewall_rule_description = "ssh"
  }

  firewall_rules {
    protocol                  = "ICMP"
    port                      = "ALL"
    cidr_block                = "0.0.0.0/0"
    action                    = "ACCEPT"
    firewall_rule_description = "ping"
  }
}

resource "tencentcloud_lighthouse_firewall_template" "empty" {
  template_name = "empty"
}
