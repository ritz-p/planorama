terraform {
  required_providers {
    aws = { source = "hashicorp/aws", version = "= 5.100.0" }
  }
}
provider "aws" {
  region = "us-east-1"
}
resource "aws_vpc" "shared" {
  cidr_block = "10.112.0.0/16"
}
resource "aws_security_group" "shared" {
  name = "shared"
  vpc_id = aws_vpc.shared.id
}
output "vpc" { value = aws_vpc.shared.id }
output "sg" { value = aws_security_group.shared.id }
