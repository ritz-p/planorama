#!/bin/sh
set -eu
node tests/support/provider_plan.cjs prepare
node tests/support/provider_plan.cjs verify
