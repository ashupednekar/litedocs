#!/usr/bin/env bash
set -euo pipefail

HOSTNAME="litedocs.server"
HOSTS_LINE="127.0.0.1 ${HOSTNAME}"
CERT_DIR="server/certs"
CERT_FILE="${CERT_DIR}/${HOSTNAME}.pem"
KEY_FILE="${CERT_DIR}/${HOSTNAME}-key.pem"

if ! command -v mkcert >/dev/null 2>&1; then
  echo "mkcert is required. Install it first (e.g. brew install mkcert nss)." >&2
  exit 1
fi

echo "[1/4] Installing local CA (mkcert -install)..."
mkcert -install

echo "[2/4] Ensuring /etc/hosts has ${HOSTS_LINE} (requires sudo)..."
if ! grep -qE "^[[:space:]]*127\.0\.0\.1[[:space:]]+${HOSTNAME}(\s|$)" /etc/hosts; then
  echo "${HOSTS_LINE}" | sudo tee -a /etc/hosts >/dev/null
  echo "Added ${HOSTS_LINE}"
else
  echo "Entry already present"
fi

echo "[3/4] Generating certs in ${CERT_DIR}..."
mkdir -p "${CERT_DIR}"
mkcert -cert-file "${CERT_FILE}" -key-file "${KEY_FILE}" "${HOSTNAME}" localhost 127.0.0.1 ::1

echo "[4/4] Done"
echo "Cert: ${CERT_FILE}"
echo "Key : ${KEY_FILE}"
echo "Set envs for server:"
echo "  export WEBAUTHN_RP_ID=${HOSTNAME}"
echo "  export WEBAUTHN_RP_ORIGIN=https://${HOSTNAME}:8443"
