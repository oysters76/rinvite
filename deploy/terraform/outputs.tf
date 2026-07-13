# ---------------------------------------------------------------------------
# ACTION REQUIRED after the first apply: set these three nameservers at your
# domain registrar. DNS + TLS then provision automatically (can take a while).
# ---------------------------------------------------------------------------
output "nameservers" {
  description = "Point your registrar's nameservers at these."
  value       = ["ns1.digitalocean.com", "ns2.digitalocean.com", "ns3.digitalocean.com"]
}

output "frontend_url" {
  description = "Your site (custom domain)."
  value       = local.frontend_url
}

output "api_url" {
  description = "The backend API + e-invite links (custom domain)."
  value       = local.api_url
}

output "frontend_default_hostname" {
  description = "App Platform's default hostname for the SPA (works before DNS cuts over)."
  value       = digitalocean_app.frontend.default_ingress
}

output "backend_default_hostname" {
  description = "App Platform's default hostname for the API (works before DNS cuts over)."
  value       = digitalocean_app.backend.default_ingress
}

output "database_host" {
  description = "Managed Postgres private host (the app connects here over the VPC)."
  value       = digitalocean_database_cluster.pg.private_host
}
