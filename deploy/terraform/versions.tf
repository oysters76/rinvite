terraform {
  required_version = ">= 1.5"

  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.43"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
  }

  # ---------------------------------------------------------------------------
  # State contains secrets (the DB URI, JWT secret). Local state is git-ignored
  # by default (see .gitignore). For a team, uncomment and use a DO Spaces
  # (S3-compatible) backend instead:
  #
  # backend "s3" {
  #   endpoints                   = { s3 = "https://nyc3.digitaloceanspaces.com" }
  #   bucket                      = "your-tfstate-bucket"
  #   key                         = "rinvite/terraform.tfstate"
  #   region                      = "us-east-1" # ignored by Spaces, but required
  #   skip_credentials_validation = true
  #   skip_metadata_api_check     = true
  #   skip_region_validation      = true
  #   skip_requesting_account_id  = true
  #   skip_s3_checksum            = true
  # }
  # ---------------------------------------------------------------------------
}
