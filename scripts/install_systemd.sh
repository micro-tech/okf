#!/usr/bin/env bash
set -e

SERVICE_NAME="okf"
SERVICE_USER="okf"
INSTALL_PATH="/usr/local/bin/okf"
WORKDIR="/opt/okf"

echo "Building OKF server..."
cargo build --release

echo "Creating dedicated service user '${SERVICE_USER}' (no login, no home)..."
if ! id -u "${SERVICE_USER}" >/dev/null 2>&1; then
    sudo useradd --system --no-create-home --shell /usr/sbin/nologin "${SERVICE_USER}"
fi

echo "Installing binary..."
sudo mkdir -p $WORKDIR
sudo cp target/release/okf $INSTALL_PATH
sudo cp -r bundles $WORKDIR/
sudo cp -r config $WORKDIR/
sudo chown -R ${SERVICE_USER}:${SERVICE_USER} $WORKDIR
sudo chmod 755 $INSTALL_PATH

echo "Creating systemd service..."
sudo tee /etc/systemd/system/${SERVICE_NAME}.service > /dev/null <<EOF
[Unit]
Description=OKF Server
After=network.target

[Service]
ExecStart=${INSTALL_PATH}
WorkingDirectory=${WORKDIR}
Restart=always
# Never run as root: the server only needs to read its bundles dir.
User=${SERVICE_USER}
Group=${SERVICE_USER}
# Harden the sandbox: no privilege escalation, read-only OS tree
# (/opt stays writable for the Phase 2 data store), private /tmp.
NoNewPrivileges=true
ProtectSystem=full
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

echo "Reloading systemd..."
sudo systemctl daemon-reload

echo "Enabling service..."
sudo systemctl enable ${SERVICE_NAME}

echo "Starting service..."
sudo systemctl start ${SERVICE_NAME}

echo "OKF server installed and running as user '${SERVICE_USER}'."
echo "NOTE: the server binds 127.0.0.1 by default. To expose it on your LAN"
echo "or tailnet, set server.host in ${WORKDIR}/config/server.yaml AND set a"
echo "bearer token via the OKF_AUTH_TOKEN environment override."
