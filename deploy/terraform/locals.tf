locals {
  api_host = "api.${var.root_domain}"

  frontend_url = "https://${var.root_domain}"
  api_url      = "https://${local.api_host}"

  # The SPA is reachable on both the apex and www; allow both as CORS origins.
  cors_origins = "https://${var.root_domain},https://www.${var.root_domain}"

  # Optional delivery secrets — only wired into the backend when non-empty, so an
  # unset provider stays in keyless log-only mode. Each entry becomes a secret
  # env var on the service.
  optional_backend_secrets = {
    RESEND_API_KEY       = var.resend_api_key
    EMAIL_FROM           = var.email_from
    TWILIO_ACCOUNT_SID   = var.twilio_account_sid
    TWILIO_AUTH_TOKEN    = var.twilio_auth_token
    TWILIO_WHATSAPP_FROM = var.twilio_whatsapp_from
    TWILIO_CONTENT_SID   = var.twilio_content_sid
  }

  backend_secret_envs = {
    for k, v in local.optional_backend_secrets : k => v if trimspace(v) != ""
  }
}
