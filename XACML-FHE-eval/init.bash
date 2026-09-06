#!/bin/bash

if ! command -v aws > /dev/null 2>&1; then
	echo "Error: could not find aws cli installed on your system (no aws command)" >&2
	exit 1
fi

if ! command -v tofu > /dev/null 2>&1; then
	echo "Error: could not find opentofu installed on your system (no tofu command)" >&2
	exit 1
fi

if ! command -v cargo > /dev/null 2>&1; then
	echo "install rust and cargo" >&2
	echo "Error: could not find cargo installed on your system (no cargo command)\n" >&2
	exit 1
fi

cargo build --bin hpdp_funcs --release

aws login --profile root-session

export AWS_PROFILE="root-session" 
export AWS_REGION="eu-west-1"

tofu init
tofu plan
tofu apply
