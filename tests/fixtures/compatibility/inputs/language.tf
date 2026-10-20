variable "fixture_secret" {
  type = string
  sensitive = true
  default = "public-fixture-placeholder"
}

resource "terraform_data" "sensitive" {
  input = var.fixture_secret
  lifecycle {
    create_before_destroy = true
    precondition {
      condition = length(var.fixture_secret) > 0
      error_message = "Fixture placeholder must be nonempty."
    }
  }
}

check "fixture" {
  assert {
    condition = terraform_data.independent.input == "independent"
    error_message = "Unexpected fixture input."
  }
}
