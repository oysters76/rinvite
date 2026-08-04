# Deploy rinvite to DigitalOcean (Terraform)

One `terraform apply` builds the full stack on **DigitalOcean App Platform**:

- **Frontend** — the SvelteKit static SPA, at `https://<your-domain>` (and `www`).
- **Backend** — the Rust API and the server-rendered e-invite pages, at `https://api.<your-domain>`.
- **Managed Postgres** — persistent storage, private (no public access).
- **DNS and TLS** — a DigitalOcean-hosted DNS zone. App Platform issues the Let's Encrypt
  certificates for both hostnames automatically.

```
                    ┌──────────────── DigitalOcean ────────────────┐
  <your-domain>  ─▶ │  App Platform: static site (web/ build)       │
  api.<domain>   ─▶ │  App Platform: service (Dockerfile, :3000) ─▶ │─▶ Managed Postgres
                    │  DNS zone + automatic TLS                     │   (private, app-only)
                    └───────────────────────────────────────────────┘
```

## Cost

Backend `basic-xxs` ≈ **$5/mo** · static site **free** · Postgres `db-s-1vcpu-1gb` ≈
**$15/mo** · DNS free → **≈ $20/mo**. Each size is a variable. Scale up in `terraform.tfvars`.

## Prerequisites (one time)

1. **A DigitalOcean account** and an API token with read and write scope
   (API → Tokens). Export it: `export DIGITALOCEAN_TOKEN=dop_v1_…` (or put it in
   `terraform.tfvars`).
2. **The repo on GitHub** (already `oysters76/rinvite`) with the **DigitalOcean GitHub app
   authorized** on it. Do this one time: DO dashboard → **Apps → Create App → GitHub → Authorize**,
   and grant access to the repo. This OAuth handshake is the only step that Terraform cannot do.
   As an alternative, if the repo is public, App Platform can build from a plain git URL (see the
   note at the end). But then you lose auto-deploy on push.
3. **A domain** bought at any registrar (DigitalOcean is *not* a registrar. Cloudflare
   Registrar, Namecheap, and Porkbun are all correct). Do not change its DNS records yet. You
   point its **nameservers** to DigitalOcean in step 4.
4. [Terraform](https://developer.hashicorp.com/terraform/install) 1.5 or later.

## Deploy

```bash
cd deploy/terraform
cp terraform.tfvars.example terraform.tfvars   # then edit: do_token, root_domain
terraform init
terraform apply
```

After the apply, Terraform prints the **nameservers**. Set them at your registrar:

```
ns1.digitalocean.com
ns2.digitalocean.com
ns3.digitalocean.com
```

DNS propagation and certificate issuance take from a few minutes to a few hours.
While you wait, the apps are already reachable at their `*_default_hostname` outputs
(the `*.ondigitalocean.app` URLs). After DNS resolves, `https://<your-domain>` and
`https://api.<your-domain>` go live with valid TLS automatically. A second apply is not
necessary.

## Everyday use

- **Ship code:** `git push` to the deploy branch. App Platform rebuilds and redeploys both
  apps automatically (`deploy_on_push`).
- **Change infra or config:** edit `*.tf` or `terraform.tfvars`, then `terraform apply`.
- **Enable real WhatsApp or email:** set the `resend_*` or `twilio_*` variables and apply again.
  Until then, the backend logs the invites instead of sending them (nothing breaks).
- **Rotate the JWT secret** (this logs everyone out): `terraform apply -replace=random_password.jwt`.
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

- **Secrets and state.** The local `terraform.tfstate` contains the DB URI and the JWT secret and is
  git-ignored. For a team, use the DO Spaces backend that is commented in `versions.tf`.
- **Region.** Keep `var.app_region` and `var.db_region` in the same datacenter, so the app and Postgres talk over the private network. They are separate variables because the two APIs name the same datacenter differently: an app uses `blr`, a database uses `blr1`.
- **Branded print PDF.** The backend image includes the card assets and `PDF_CONFIG`, so the
  floral-gold PDF renders in production with no extra config.
- **Public-repo alternative to GitHub OAuth.** Replace the `github { … }` blocks in
  `backend.tf` and `frontend.tf` with `git { repo_clone_url = "https://github.com/oysters76/rinvite.git", branch = "master" }`.
  No authorization is necessary, but there is no automatic redeploy on push.
