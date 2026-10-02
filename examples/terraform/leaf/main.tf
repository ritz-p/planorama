variable "input" {
  type = string
}

resource "terraform_data" "leaf" {
  input = var.input
}

output "id" {
  value = terraform_data.leaf.id
}
