variable "cidr" {
  type = string
}
locals {
  subnet_cidrs = ["10.0.1.0/24", "10.0.2.0/24"]
}
resource "aws_vpc" "main" {
  cidr_block = var.cidr
}
resource "aws_subnet" "private" {
  count             = 2
  vpc_id            = aws_vpc.main.id
  cidr_block        = local.subnet_cidrs[count.index]
  availability_zone = ["us-east-1a", "us-east-1b"][count.index]
}
resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id
}
resource "aws_route_table_association" "private" {
  count          = 2
  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}
output "subnet_ids" {
  value = aws_subnet.private[*].id
}
