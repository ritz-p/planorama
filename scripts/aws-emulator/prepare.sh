#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
tools=${PLANORAMA_EMULATOR_TOOLS:-"$PWD/output/aws-emulator-tools"}
case "$(uname -s):$(uname -m)" in Linux:x86_64) ;; *) echo 'Pinned sample tools require Linux amd64' >&2; exit 1;; esac
mkdir -p "$tools"
tools=$(realpath "$tools")
exec 9>"$tools/install.lock"
flock 9
stage=$(mktemp -d "$tools/.install.XXXXXXXX")
trap 'rm -rf -- "$stage"' EXIT
download() {
    file=$1
    curl --fail --silent --show-error --location --retry 2 --proto '=https' --proto-redir '=https' "$2" -o "$stage/$file"
    (cd "$stage" && echo "$3  $file" | sha256sum -c -)
}
download terraform.zip https://releases.hashicorp.com/terraform/1.9.8/terraform_1.9.8_linux_amd64.zip 186e0145f5e5f2eb97cbd785bc78f21bae4ef15119349f6ad4fa535b83b10df8
download aws.zip https://releases.hashicorp.com/terraform-provider-aws/6.70.0/terraform-provider-aws_6.70.0_linux_amd64.zip 58acc041b080778ded75320e241ebcf6cf2ed145bede644b86a4784ce46086c6
download emulator.deb https://downloads.aws.com/aws-emulator/packages/aws-emulator/LTS/aws-emulator_1.0.508-1_amd64.deb a906dbbd84b8217a57d6067f44b4bf97d72dad64c85cbbbce11991deb8264c2f
unzip -q "$stage/terraform.zip" terraform -d "$stage"
dpkg-deb -x "$stage/emulator.deb" "$stage/package"
provider=mirror/registry.terraform.io/hashicorp/aws/6.70.0/linux_amd64
mkdir -p "$stage/$provider" "$tools/$provider"
unzip -q "$stage/aws.zip" -d "$stage/$provider"
mv "$stage/terraform" "$tools/terraform"
mv "$stage/package/opt/aws-emulator/bin/aws-emulator" "$tools/aws-emulator"
mv "$stage/$provider/LICENSE.txt" "$tools/$provider/"
mv "$stage/$provider/terraform-provider-aws_v6.70.0_x5" "$tools/$provider/"

if [ "${1:-}" = large ]; then
    download aws5.zip https://releases.hashicorp.com/terraform-provider-aws/5.100.0/terraform-provider-aws_5.100.0_linux_amd64.zip 1589a2266af699cbd5d80737a0fe02e54ec9cf2ca54e7e00ac51c7359056f274
    provider=mirror/registry.terraform.io/hashicorp/aws/5.100.0/linux_amd64
    mkdir -p "$stage/$provider" "$tools/$provider"
    unzip -q "$stage/aws5.zip" -d "$stage/$provider"
    mv "$stage/$provider/LICENSE.txt" "$stage/$provider/terraform-provider-aws_v5.100.0_x5" "$tools/$provider/"
fi
