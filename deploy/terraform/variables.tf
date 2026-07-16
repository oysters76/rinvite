variable "do_token" {
  description = "DigitalOcean API token (read/write). Prefer the DIGITALOCEAN_TOKEN env var or a *.auto.tfvars file kept out of git."
  type        = string
  sensitive   = true
}

variable "root_domain" {
  description = "Your custom apex domain, e.g. \"example.com\". The SPA is served here (+ www); the API at api.<root_domain>."
  type        = string
}

variable "region" {
  description = "DigitalOcean region slug. The app and its Postgres cluster must share a region for private networking."
  type        = string
  default     = "nyc3"
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
