resource "terraform_data" "endpoint" {
  input = "synthetic-id"
}
resource "terraform_data" "consumer" {
  input = {
    account = { id = terraform_data.endpoint.id }
    bindings = [{ name = "KV", type = "kv_namespace", namespace_id = terraform_data.endpoint.id }]
  }
}
