variable "upstream" {
  type = string
}

resource "terraform_data" "worker" {
  count = 2
  input = var.upstream
}

module "nested" {
  source = "../leaf"
  input  = terraform_data.worker[0].id
}

output "id" {
  value = module.nested.id
}
