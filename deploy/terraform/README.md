# Deploying rinvite to DigitalOcean (Terraform)

One `terraform apply` stands up the whole thing on **DigitalOcean App Platform**:

- **Frontend** — the SvelteKit static SPA, served at `https://<your-domain>` (+ `www`).
- **Backend** — the Rust API + server-rendered e-invite pages, at `https://api.<your-domain>`.
- **Managed Postgres** — persistent storage, private (no public access).
- **DNS + TLS** — a DigitalOcean-hosted DNS zone; App Platform auto-issues Let's Encrypt
  certificates for both hostnames.

```
                    ┌──────────────── DigitalOcean ────────────────┐
  <your-domain>  ─▶ │  App Platform: static site (web/ build)       │
  api.<domain>   ─▶ │  App Platform: service (Dockerfile, :3000) ─▶ │─▶ Managed Postgres
                    │  DNS zone + automatic TLS                     │   (private, app-only)
                    └───────────────────────────────────────────────┘
```

## Cost

Backend `basic-xxs` ≈ **$5/mo** · static site **free** · Postgres `db-s-1vcpu-1gb` ≈
**$15/mo** · DNS free → **≈ $20/mo**. Every size is a variable — scale up in `terraform.tfvars`.

## Prerequisites (one-time)

1. **A DigitalOcean account** and an API token with read/write scope
   (API → Tokens). Export it: `export DIGITALOCEAN_TOKEN=dop_v1_…` (or put it in
   `terraform.tfvars`).
2. **The repo on GitHub** (already `oysters76/rinvite`) with the **DigitalOcean GitHub app
   authorized** on it. Do this once: DO dashboard → **Apps → Create App → GitHub → Authorize**,
   and grant access to the repo. (This OAuth handshake is the only step Terraform can't do.
   Alternatively, if the repo is public, App Platform can build from a plain git URL — see
   note at the bottom — but you lose auto-deploy-on-push.)
3. **A domain** bought at any registrar (DigitalOcean is *not* a registrar — Cloudflare
   Registrar, Namecheap, and Porkbun are all fine). Don't touch its DNS records yet; you'll
   repoint its **nameservers** to DigitalOcean in step 4.
4. [Terraform](https://developer.hashicorp.com/terraform/install) ≥ 1.5.

## Deploy

```bash
cd deploy/terraform
cp terraform.tfvars.example terraform.tfvars   # then edit: do_token, root_domain
terraform init
terraform apply
```

After apply, Terraform prints the **nameservers**. Set them at your registrar:

```
ns1.digitalocean.com
ns2.digitalocean.com
ns3.digitalocean.com
```

DNS propagation + certificate issuance can take anywhere from a few minutes to a few hours.
While you wait, the apps are already reachable at their `*_default_hostname` outputs
(the `*.ondigitalocean.app` URLs). Once DNS resolves, `https://<your-domain>` and
`https://api.<your-domain>` go live with valid TLS automatically — no second apply needed.

## Everyday use

- **Ship code:** `git push` to the deploy branch → App Platform rebuilds and redeploys both
  apps automatically (`deploy_on_push`).
- **Change infra/config:** edit `*.tf` / `terraform.tfvars` → `terraform apply`.
- **Enable real WhatsApp/email:** set the `resend_*` / `twilio_*` variables and re-apply.
  Until then the backend logs invites instead of sending them (nothing breaks).
- **Rotate the JWT secret** (logs everyone out): `terraform apply -replace=random_password.jwt`.
- **Tear it all down:** `terraform destroy`.

## What's where

| File | Purpose |
|---|---|
| `versions.tf` | Terraform + provider version pins (and a commented Spaces remote-state backend). |
| `providers.tf` | DigitalOcean provider. |
| `variables.tf` | All inputs (domain, region, sizes, optional delivery secrets). |
| `locals.tf` | Derived hostnames, CORS origins, optional-secret filtering. |
| `dns.tf` | The DigitalOcean DNS zone. |
| `database.tf` | Managed Postgres cluster. |
| `secrets.tf` | Generated `JWT_SECRET`. |
| `backend.tf` | The API app (Docker service + attached DB + env). |
| `frontend.tf` | The SPA app (static site + build-time API URL). |
| `outputs.tf` | Nameservers, live URLs, default hostnames. |

## Notes

- **Secrets & state.** Local `terraform.tfstate` contains the DB URI and JWT secret and is
  git-ignored. For a team, use the DO Spaces backend commented in `versions.tf`.
- **Region.** The app and Postgres share `var.region` so they talk over the private network.
- **Branded print PDF.** The backend image bakes in the card assets and `PDF_CONFIG`, so the
  floral-gold PDF renders in production out of the box (no extra config).
- **Public-repo alternative to GitHub OAuth.** Replace the `github { … }` blocks in
  `backend.tf`/`frontend.tf` with `git { repo_clone_url = "https://github.com/oysters76/rinvite.git", branch = "master" }`.
  No authorization needed, but no automatic redeploy on push.
