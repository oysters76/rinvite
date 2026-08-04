variable "do_token" {
  description = "DigitalOcean API token (read/write). Prefer the DIGITALOCEAN_TOKEN env var or a *.auto.tfvars file kept out of git."
  type        = string
  sensitive   = true
}

variable "root_domain" {
  description = "Your custom apex domain, e.g. \"example.com\". The SPA is served here (+ www); the API at api.<root_domain>."
  type        = string
}

variable "invite_base_url" {
  description = "Public base URL for guest-facing invite links — a reverse proxy in front of the API's /i/* routes. Links look like <invite_base_url>/i/<token>. Empty = point links at the API host directly."
  type        = string
  default     = "https://rinvite.ceykod.com"
}

# App Platform and managed databases use different region slugs for the same
# datacenter — Bangalore is "blr" for an app and "blr1" for a cluster — so they
# need separate variables. Keep both in the same datacenter: the app reaches the
# cluster over the private network, and a mismatch sends every query across the
# public internet instead.
variable "app_region" {
  description = "App Platform region slug for the API and web apps, e.g. \"blr\" (Bangalore) or \"nyc\" (New York). Note App Platform slugs carry no trailing digit."
  type        = string
  default     = "blr"
}

variable "db_region" {
  description = "Managed-database region slug for the Postgres cluster, e.g. \"blr1\" (Bangalore) or \"nyc3\" (New York). Must be the same datacenter as app_region. Changing it makes DigitalOcean migrate the cluster online — the data is kept, but the host changes and connections fail over, so expect a short interruption."
  type        = string
  default     = "blr1"
}

variable "github_repo" {
  description = "owner/name of the GitHub repo App Platform builds from. The DigitalOcean GitHub app must be authorized on it (one-time, in the DO dashboard)."
  type        = string
  default     = "oysters76/rinvite"
}

variable "github_branch" {
  description = "Branch App Platform deploys (and auto-redeploys on push)."
  type        = string
  default     = "master"
}

variable "service_instance_size" {
  description = "Instance size slug for the backend service. basic-xxs is the smallest/cheapest."
  type        = string
  default     = "basic-xxs"
}

variable "db_size" {
  description = "Managed Postgres node size slug. db-s-1vcpu-1gb is the smallest managed tier."
  type        = string
  default     = "db-s-1vcpu-1gb"
}

variable "db_version" {
  description = "Postgres major version."
  type        = string
  default     = "16"
}

# ---- Optional e-invite delivery secrets ------------------------------------
# Leave empty to run in keyless mode: the backend logs invites instead of
# sending them (no Resend/Twilio account required for the first deploy).

variable "resend_api_key" {
  description = "Resend API key for sending emails. Empty = log-only."
  type        = string
  default     = ""
  sensitive   = true
}

variable "email_from" {
  description = "Sender for emails, e.g. \"Rinvite <invites@example.com>\". Required to actually send email."
  type        = string
  default     = ""
}

variable "twilio_account_sid" {
  description = "Twilio Account SID for WhatsApp. Empty = log-only."
  type        = string
  default     = ""
  sensitive   = true
}

variable "twilio_auth_token" {
  description = "Twilio Auth Token."
  type        = string
  default     = ""
  sensitive   = true
}

variable "twilio_whatsapp_from" {
  description = "WhatsApp-enabled sender in E.164, e.g. +14155238886 (Twilio sandbox number)."
  type        = string
  default     = ""
}

variable "twilio_content_sid" {
  description = "Meta-approved template ContentSid, required for production business-initiated WhatsApp. Empty = freeform (sandbox/24h window)."
  type        = string
  default     = ""
  sensitive   = true
}

variable "twilio_sms_from" {
  description = "SMS-capable Twilio sender in E.164, e.g. +14155238886. Reuses twilio_account_sid/twilio_auth_token above. Empty = SMS log-only."
  type        = string
  default     = ""
}

# ---- Optional contact addresses --------------------------------------------

variable "business_contact_email" {
  description = "Contact shown to users in the plan \"limit reached\" dialog. Empty = the app's built-in default."
  type        = string
  default     = ""
}

variable "upgrade_notify_email" {
  description = "Where plan upgrade-request notifications are delivered. Empty = falls back to business_contact_email."
  type        = string
  default     = ""
}
