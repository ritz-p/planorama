terraform {
  required_version = "= 1.9.8"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "= 6.70.0"
    }
  }
}

provider "aws" {
  region = "us-east-1"
  endpoints {
    ec2 = "https://127.0.0.1:9297"
    sts = "https://127.0.0.1:9297"
  }
}

resource "aws_vpc" "main" {
  cidr_block = "10.82.0.0/16"
}

resource "aws_subnet" "private" {
  availability_zone = "us-east-1a"
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.82.1.0/24"
}

resource "aws_instance" "app" {
  availability_zone = "us-east-1a"
  ami               = "ami-0123456789abcdef0"
  instance_type     = "t3.micro"
  subnet_id         = aws_subnet.private.id
  tags              = { Network = aws_vpc.main.id }
}
