#!/bin/sh
set -eu
export TF_IN_AUTOMATION=1
export TF_DATA_DIR=/tmp/planorama-terraform
mkdir -p /workspace/output
terraform -chdir=examples/terraform init -backend=false -input=false -no-color
terraform -chdir=examples/terraform plan -input=false -lock=false -no-color -out=/workspace/output/example.tfplan
terraform -chdir=examples/terraform show -json /workspace/output/example.tfplan > /workspace/output/terraform-plan.json
