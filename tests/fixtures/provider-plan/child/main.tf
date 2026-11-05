terraform {
  required_providers {
    aws = { source = "hashicorp/aws" }
  }
}

resource "aws_vpc" "fixture" {
  cidr_block = "10.1.0.0/16"
}
