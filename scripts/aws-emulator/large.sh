#!/bin/sh
set -eu
repo=$1 tools=$2 binary=$3 artifact=$4
mkdir seed
cp "$repo/examples/aws-emulator/large-seed/main.tf" seed/
source_dir="$repo/examples/terraform-large"
cp "$source_dir/.terraform.lock.hcl" seed/
cp -r "$source_dir/main.tf.json" "$source_dir/.terraform.lock.hcl" "$source_dir/shared" "$source_dir/application" "$source_dir/database" "$source_dir/network" module/
seed() { timeout 5m "$tools/terraform" -chdir=seed "$@"; }
tf() { timeout 5m "$tools/terraform" -chdir=module "$@"; }
seed init -lockfile=readonly -input=false -no-color
seed apply -auto-approve -input=false -no-color
vpc=$(seed output -raw vpc)
group=$(seed output -raw sg)
tf init -lockfile=readonly -input=false -no-color
tf plan -input=false -no-color -var="shared_vpc_id=$vpc" -var="shared_security_group_id=$group" -out="$artifact/create.tfplan"
tf show -json "$artifact/create.tfplan" >"$artifact/create.json"
for format in svg html; do
    "$binary" "$artifact/create.json" --format "$format" -o "$artifact/create.$format"
done
node "$repo/scripts/aws-emulator/stress-base.cjs" "$artifact/create.json" >"$artifact/stress.json"
seed destroy -auto-approve -input=false -no-color
