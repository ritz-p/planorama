terraform {
  required_version = "= 1.9.8"
  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "= 5.28.1"
    }
  }
}

provider "cloudflare" {}

variable "revision" {
  type    = string
  default = "initial"
}

locals {
  account = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}

resource "cloudflare_zone" "example" {
  account = { id = local.account }
  name    = "example.invalid"
  type    = "full"
}

resource "cloudflare_dns_record" "www" {
  zone_id = cloudflare_zone.example.id
  name    = "www.example.invalid"
  type    = "A"
  content = "192.0.2.1"
  ttl     = var.revision == "initial" ? 300 : 600
}

resource "cloudflare_worker" "api" {
  account_id = local.account
  name       = "fixture-api"
}

resource "cloudflare_workers_kv_namespace" "cache" {
  account_id = local.account
  title      = "fixture-cache-${var.revision}"
}

resource "cloudflare_r2_bucket" "assets" {
  account_id   = local.account
  name         = "fixture-assets"
  jurisdiction = "eu"
}

resource "cloudflare_d1_database" "content" {
  account_id = local.account
  name       = "fixture-content"
}

resource "cloudflare_worker_version" "api" {
  account_id  = local.account
  worker_id   = cloudflare_worker.api.id
  main_module = "main.js"
  modules = [{
    name           = "main.js"
    content_type   = "application/javascript+module"
    content_base64 = base64encode("export default {fetch() {return new Response('fixture-only');}};")
  }]
  bindings = [
    { name = "CACHE", type = "kv_namespace", namespace_id = cloudflare_workers_kv_namespace.cache.id },
    { name = "ASSETS", type = "r2_bucket", bucket_name = cloudflare_r2_bucket.assets.name, jurisdiction = "eu" },
    { name = "DB", type = "d1", id = cloudflare_d1_database.content.id },
  ]
}
