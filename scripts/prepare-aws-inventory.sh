#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
source_dir=$(mktemp -d /tmp/planorama-aws-source.XXXXXX)
trap 'rm -rf "$source_dir"' EXIT HUP INT TERM
curl -fsSL https://api.github.com/repos/hashicorp/terraform-provider-aws/tarball/5e13eedee7c22ac362e6ffdfbdcfd23a5f9f797f | tar xz --strip-components=1 -C "$source_dir"
mkdir -p /tmp/aws-inventory-schema
cat > /tmp/aws-inventory-schema/main.tf <<'EOF'
terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "6.70.0"
    }
  }
}
EOF
terraform -chdir=/tmp/aws-inventory-schema init -backend=false -input=false
terraform -chdir=/tmp/aws-inventory-schema providers schema -json > /tmp/aws-inventory-schema/schema.json
node scripts/aws-inventory.cjs "$source_dir" /tmp/aws-inventory-schema/schema.json /tmp/aws-inventory-schema/inventory.json
cmp tests/fixtures/aws-inventory/6.70.0.json /tmp/aws-inventory-schema/inventory.json
