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
    sqs = "https://127.0.0.1:9297"
    sts = "https://127.0.0.1:9297"
  }
}

variable "delay_seconds" {
  type    = number
  default = 0
}

resource "aws_sqs_queue" "jobs" {
  name          = "planorama-emulator-jobs"
  delay_seconds = var.delay_seconds
}
