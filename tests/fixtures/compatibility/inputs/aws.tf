resource "aws_ecs_task_definition" "fixture" {
  family = "planorama-compatibility"
  container_definitions = jsonencode([{
    name = "fixture"
    image = "public.ecr.aws/docker/library/busybox:1.36"
    environment = [{ name = "NETWORK", value = module.network.subnet_ids[0] }]
    memory = 128
    essential = true
  }])
}

resource "aws_security_group" "dynamic" {
  name = "planorama-compatibility"
  dynamic "ingress" {
    for_each = [80, 443]
    content {
      from_port = ingress.value
      to_port = ingress.value
      protocol = "tcp"
      cidr_blocks = ["10.0.0.0/16"]
    }
  }
}
