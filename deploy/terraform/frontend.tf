# Frontend: the SvelteKit static SPA, built from web/ and served at the apex
# (+ www). VITE_API_BASE_URL is baked in at BUILD time so the SPA calls the API.
resource "digitalocean_app" "frontend" {
  spec {
    name   = "rinvite-web"
    region = var.region

    domain {
      name = var.root_domain
      type = "PRIMARY"
      zone = digitalocean_domain.main.name
    }
    domain {
      name = "www.${var.root_domain}"
      type = "ALIAS"
      zone = digitalocean_domain.main.name
    }

    static_site {
      name          = "web"
      source_dir    = "/web"
      build_command = "npm ci && npm run build"
      output_dir    = "build"
      # SPA deep-link fallback: serve index.html for any unmatched path so the
      # client router handles routes like /events/123.
      catchall_document = "index.html"

      github {
        repo           = var.github_repo
        branch         = var.github_branch
        deploy_on_push = true
      }

      # Baked into the bundle at build time (see web/src/lib/api/client.ts).
      env {
        key   = "VITE_API_BASE_URL"
        value = local.api_url
        scope = "BUILD_TIME"
      }
    }
  }
}
