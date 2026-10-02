terraform {
  required_version = ">= 1.9.0, < 2.0.0"
}

# Uses only Terraform's built-in provider. No credentials or cloud resources.
resource "terraform_data" "network" {
  input = { names = ["api", "web"] }
}

resource "terraform_data" "ready" {
  input = "ready"
}

module "service" {
  source     = "./service"
  for_each   = toset(terraform_data.network.input.names)
  upstream   = terraform_data.network.id
  depends_on = [terraform_data.ready]
}

module "replica" {
  source = "./leaf"
  count  = length(terraform_data.network.input.names)
  input  = terraform_data.ready.id
}

resource "terraform_data" "consumer" {
  input = [for service in module.service : service.id]
}

resource "terraform_data" "independent" {
  input = "independent"
}

resource "terraform_data" "explicit" {
  depends_on = [terraform_data.independent]
}

# Read during plan from a checked-in, synthetic local state snapshot.
data "terraform_remote_state" "existing" {
  backend = "local"
  config = {
    path = "${path.module}/shared-state.json"
  }
}

# The dependency on a resource being created defers this read until apply.
data "terraform_remote_state" "after_ready" {
  backend = "local"
  config = {
    path = "${path.module}/shared-state.json"
  }
  depends_on = [terraform_data.ready]
}

resource "terraform_data" "from_state" {
  input = {
    existing = data.terraform_remote_state.existing.outputs.network_id
    deferred = data.terraform_remote_state.after_ready.outputs.network_id
  }
}
