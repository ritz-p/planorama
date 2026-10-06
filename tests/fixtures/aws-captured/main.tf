terraform {
  required_version = "= 1.9.8"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "= 5.100.0"
    }
  }
}
provider "aws" {
  region                      = "us-east-1"
  skip_credentials_validation = true
  skip_requesting_account_id  = true
  skip_metadata_api_check     = true
}
module "network" {
  source = "./network"
  cidr   = "10.0.0.0/16"
}
resource "aws_lb" "app" {
  name               = "planorama-fixture"
  internal           = true
  load_balancer_type = "application"
  subnets            = module.network.subnet_ids
}
data "aws_iam_policy_document" "fixture" {
  statement {
    actions   = ["s3:ListAllMyBuckets"]
    resources = ["*"]
  }
}
output "policy" {
  value = data.aws_iam_policy_document.fixture.json
}
