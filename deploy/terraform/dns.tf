# The DNS zone for your domain, hosted on DigitalOcean. After the first apply,
# set your registrar's nameservers to the ones in the `nameservers` output.
#
# We do NOT declare individual A/CNAME records here: when an App Platform app's
# `domain` block references a zone in this same account, App Platform creates and
# manages the records (and the TLS certificate) automatically.
resource "digitalocean_domain" "main" {
  name = var.root_domain
}
