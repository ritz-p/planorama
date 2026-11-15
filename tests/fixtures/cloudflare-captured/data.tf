data "cloudflare_zone" "existing" {
  zone_id = "cccccccccccccccccccccccccccccccc"
}

data "cloudflare_zone" "deferred" {
  zone_id = cloudflare_zone.example.id
}

resource "cloudflare_dns_record" "lookup" {
  zone_id = data.cloudflare_zone.existing.id
  name    = "lookup.example.invalid"
  type    = "A"
  content = "192.0.2.2"
  ttl     = 300
}

resource "cloudflare_dns_record" "deferred" {
  zone_id = data.cloudflare_zone.deferred.id
  name    = "deferred.example.invalid"
  type    = "A"
  content = "192.0.2.3"
  ttl     = 300
}
