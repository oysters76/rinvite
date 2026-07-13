# Backend: the Rust/Axum service, built from the repo Dockerfile and served at
# api.<root_domain>. It also server-renders the e-invite pages (/i/{token}), so
# invite links point here (PUBLIC_BASE_URL).
resource "digitalocean_app" "backend" {
  spec {
    name   = "rinvite-api"
    region = var.region

    domain {
      name = local.api_host
      type = "PRIMARY"
      zone = digitalocean_domain.main.name
    }

    service {
      name               = "api"
      instance_count     = 1
      instance_size_slug = var.service_instance_size
      http_port          = 3000

      github {
        repo           = var.github_repo
        branch         = var.github_branch
        deploy_on_push = true
      }
      dockerfile_path = "Dockerfile"

      health_check {
        http_path = "/health"
      }

      # ---- Required runtime config -----------------------------------------
      env {
        key   = "JWT_SECRET"
        value = random_password.jwt.result
        type  = "SECRET"
        scope = "RUN_TIME"
      }
      env {
        key   = "DATABASE_URL"
        value = "$${db.DATABASE_URL}" # bound from the attached cluster below
        type  = "SECRET"
        scope = "RUN_TIME"
      }
      env {
        key   = "PUBLIC_BASE_URL"
        value = local.api_url
        scope = "RUN_TIME"
      }
      env {
        key   = "CORS_ALLOWED_ORIGINS"
        value = local.cors_origins
        scope = "RUN_TIME"
      }

      # ---- Optional delivery secrets (only present when set) ----------------
      dynamic "env" {
        for_each = local.backend_secret_envs
        content {
          key   = env.key
          value = env.value
          type  = "SECRET"
          scope = "RUN_TIME"
        }
      }
    }

    # Attach the managed Postgres cluster. `name = "db"` is what the
    # `$${db.DATABASE_URL}` binding above refers to.
    database {
      name         = "db"
      engine       = "PG"
      cluster_name = digitalocean_database_cluster.pg.name
      production   = true
    }
  }
}
