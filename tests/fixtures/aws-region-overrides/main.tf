terraform {
  required_version = "= 1.9.8"
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "= 6.0.0"
    }
  }
}
provider "aws" {
  region = "us-east-1"
  skip_credentials_validation = true
  skip_requesting_account_id = true
  skip_metadata_api_check = true
}
resource "aws_vpc" "paris" {
  region = "eu-west-3"
  cidr_block = "10.0.0.0/16"
}
resource "aws_subnet" "paris" {
  region = "eu-west-3"
  vpc_id = aws_vpc.paris.id
  cidr_block = "10.0.1.0/24"
}
resource "aws_cloudwatch_log_group" "paris" {
  region = "eu-west-3"
  name = "planorama-region-fixture"
}
resource "aws_vpc" "default" {
  cidr_block = "10.1.0.0/16"
}
