# Large-fixture route comparison

Comparison for the #291 refinement: initial PR commit `a7cedcb` versus the revised
lane-planning implementation. Both use the same fixture, Rust 1.85.1 Docker
container and optimized test profile. Two layout runs measured 5.898 / 5.707 s
before and 7.219 / 7.036 s after; this is about 1.3 s additional work for bounded
rerouting, not a performance improvement or a CI timing threshold.

All 37 final polylines were measured after embedded Route reconstruction and
shortcutting. Edge reversal reproduces the same shapes. Length and backtracking
are pixels; stretch is length divided by the shortest valid reference with the
same ports and marker clearance. That reference ignores foreign-card occlusion;
the focused private-subnet checks additionally protect those cards and containers.

An offset is a short perpendicular step between longer same-direction legs
(step < 32px and less than half of each leg). An S-jog pairs opposite offsets
separated by one leg. Reversals count changes of direction on either axis;
backtracking sums the smaller positive/negative travel on each axis. These
describe shapes, not universal prohibitions: fixed endpoints, hard obstacles and
marker reservations can require a short step.

Values are **initial → revised**; a single value means unchanged. Addresses omit
only the common `module.` prefix. Every S-jog count is zero in both versions;
aggregate counts alone therefore did not identify the visual regression.
The private-b subnet → DB subnet group route loses two unnecessary bends.
The two remaining offset-shaped routes (DB subnet group → DB instance and shared
SG → ECS) are already shortest-length, two-bend routes for their fixed ports.
At #291, private-a subnet → private Route Table retained a 29px reversal because
the previous graph-wide reservation imposed a 50px approach on its numbered top
port. #322 uses the actual badge height and a 36px top/bottom approach; the table
below records the historical #291 geometry. Private-b's three-bend Route
Table route avoids the private-a container; the occlusion-blind one-bend reference
passes behind that container. The NAT Route's backtracking increases by 2px with
the changed geometry while total length falls by 50px; it retains five bends and
one reversal. The final per-edge acceptance also guards against worsening shape
relative to the unspaced geometry used by that run.

| Source | Target | Length | Bends | Offsets | Reversals | Backtracking | S-jogs | Stretch |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| application.aws_lb.app | application.aws_lb_listener.http | 763 → 757 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_lb_target_group.app | application.aws_instance.workers_a[0] | 1207 → 1153 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_lb_target_group.app | application.aws_instance.workers_a[1] | 1441 → 1387 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_lb_target_group.app | application.aws_instance.workers_b[0] | 1955 → 1901 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_lb_target_group.app | application.aws_instance.workers_b[1] | 2189 → 2135 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_lb_target_group.app | application.aws_lb_listener.http | 1057 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.alb | application.aws_lb.app | 925 → 919 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.alb | application.aws_security_group.app | 553 → 547 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | application.aws_ecs_service.app | 1584 → 1536 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | application.aws_instance.workers_a[0] | 286 → 238 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | application.aws_instance.workers_a[1] | 404 → 356 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | application.aws_instance.workers_b[0] | 913 → 865 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | application.aws_instance.workers_b[1] | 1171 → 1123 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| application.aws_security_group.app | database.aws_security_group.database | 619 → 571 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| database.aws_db_subnet_group.database | database.aws_db_instance.database | 435 | 2 | 1 | 0 | 0 | 0 | 1.000 |
| database.aws_security_group.database | database.aws_db_instance.database | 1567 | 2 | 0 | 1 | 16 | 0 | 1.000 |
| network.aws_eip.nat | network.aws_nat_gateway.main | 305 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_internet_gateway.main | network.aws_nat_gateway.main | 1087 | 1 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_route.internet | network.aws_internet_gateway.main | 2003 → 1997 | 3 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_route.nat | network.aws_nat_gateway.main | 1972 → 1922 | 5 | 0 | 1 | 83 → 85 | 0 | 1.092 → 1.097 |
| network.aws_security_group.endpoints | network.aws_vpc_endpoint.logs | 84 | 0 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_a | application.aws_ecs_service.app | 395 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_a | database.aws_db_subnet_group.database | 402 | 1 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_a | network.aws_route_table.private | 281 | 3 | 0 | 1 | 29 | 0 | 1.000 |
| network.aws_subnet.private_a | network.aws_vpc_endpoint.logs | 325 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_b | application.aws_ecs_service.app | 492 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_b | database.aws_db_subnet_group.database | 1148 | 4 → 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_b | network.aws_route_table.private | 762 | 3 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.private_b | network.aws_vpc_endpoint.logs | 636 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.public_a | application.aws_lb.app | 201 → 195 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.public_a | network.aws_route_table.public | 523 → 517 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.public_b | application.aws_lb.app | 287 → 281 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_subnet.public_b | network.aws_route_table.public | 983 → 977 | 3 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_vpc.main | network.aws_internet_gateway.main | 894 → 865 | 1 | 0 | 0 | 0 | 0 | 1.000 |
| network.aws_vpc.main | network.aws_vpc_peering_connection.shared | 505 | 2 | 0 | 0 | 0 | 0 | 1.000 |
| shared.data.aws_security_group.shared | application.aws_ecs_service.app | 589 | 2 | 1 | 0 | 0 | 0 | 1.000 |
| shared.data.aws_vpc.shared | network.aws_vpc_peering_connection.shared | 4073 → 4019 | 3 | 0 | 1 | 188 | 0 | 1.092 → 1.094 |

