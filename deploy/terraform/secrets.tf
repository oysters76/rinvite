# The backend refuses to boot without a >=32-byte JWT_SECRET. Generate a strong
# one and let App Platform store it encrypted (see backend.tf). Rotating it =
# `terraform taint random_password.jwt && terraform apply` (logs everyone out).
resource "random_password" "jwt" {
  length  = 48
  special = false
}
