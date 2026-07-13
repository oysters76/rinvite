# Managed Postgres — persists events, guests, and RSVPs across redeploys. The
# backend runs its embedded sqlx migrations on boot, so there's no migration job.
#
# The backend app attaches to this cluster via a `database` block in its spec
# (see backend.tf). That attachment makes App Platform:
#   - automatically add the app to the cluster's TRUSTED SOURCES (so Postgres
#     stays off the public internet — no separate firewall resource, and no
#     app-vs-firewall create-order race), and
#   - inject a private-network, TLS (`sslmode=require`) connection string that we
#     pass through as DATABASE_URL.
resource "digitalocean_database_cluster" "pg" {
  name       = "rinvite-pg"
  engine     = "pg"
  version    = var.db_version
  size       = var.db_size
  region     = var.region
  node_count = 1
}
