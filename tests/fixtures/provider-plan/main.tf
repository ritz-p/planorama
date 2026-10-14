terraform {
  required_version = "= 1.9.8"
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "= 5.100.0"
    }
  }
}

variable "sts_endpoint" { type = string }

provider "aws" {
  alias = "verified"
  region = "us-east-1"
  skip_credentials_validation = true
  skip_requesting_account_id = true
  skip_metadata_api_check = true
  skip_region_validation = true
  endpoints { sts = var.sts_endpoint }
  assume_role {
    role_arn = "arn:aws:iam::123456789012:role/Fixture"
  }
}

resource "aws_vpc" "fixture" {
  provider = aws.verified
  cidr_block = "10.0.0.0/16"
}
