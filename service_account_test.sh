#!/usr/bin/env bash

set -eux pipefail

export ENVIRONMENT=localhost
export PROTON_PASS_USERNAME=passuitest
export PROTON_PASS_PASSWORD=a

./target/debug/pass-cli logout --force
./target/debug/pass-cli login --interactive "${PROTON_PASS_USERNAME}"

VAULT_NAME=$(dd if=/dev/urandom bs=1 count=2048 2>/dev/null | sha256sum | awk '{print $1}' | cut -c1-6)

VAULT_NAME_1="v1_${VAULT_NAME}"
VAULT_NAME_2="v2_${VAULT_NAME}"

./target/debug/pass-cli vault create --name "${VAULT_NAME_1}"
./target/debug/pass-cli vault create --name "${VAULT_NAME_2}"

ITEM_TITLE_1="ShareMe_Vault1"
ITEM_TITLE_2="ShareMe_Vault2"

ITEM_1_ID=$(./target/debug/pass-cli item create note --vault-name "${VAULT_NAME_1}" --note "This is a note" --title "${ITEM_TITLE_1}")
ITEM_2_ID=$(./target/debug/pass-cli item create note --vault-name "${VAULT_NAME_2}" --note "This is a note" --title "${ITEM_TITLE_2}")

SERVICE_ACCOUNT_NAME="MyServiceAccount_${VAULT_NAME}"
SERVICE_ACCOUNT_RESPONSE=$(./target/debug/pass-cli internal service-account create --name "${SERVICE_ACCOUNT_NAME}" --output "json")

SERVICE_ACCOUNT_TOKEN=$(echo "${SERVICE_ACCOUNT_RESPONSE}" | jq -r '.token')
SERVICE_ACCOUNT_ID=$(echo "${SERVICE_ACCOUNT_RESPONSE}" | jq -r '.service_account_id')

./target/debug/pass-cli internal service-account access grant --service-account-id "${SERVICE_ACCOUNT_ID}" --vault-name "${VAULT_NAME_1}"
./target/debug/pass-cli internal service-account access grant --service-account-id "${SERVICE_ACCOUNT_ID}" --vault-name "${VAULT_NAME_2}" --item-title "${ITEM_TITLE_2}"

echo "${SERVICE_ACCOUNT_TOKEN}"