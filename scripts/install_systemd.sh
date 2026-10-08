

#!/usr/bin/env bash
set -e

SERVICE_NAME="okf"
INSTALL_PATH="/usr/local/bin/okf"
WORKDIR="/opt/okf"

echo "Building OKF server..."
cargo build --release

echo "Installing binary..."
sudo mkdir -p $WORKDIR
sudo cp target/release/okf $INSTALL_PATH
sudo cp -r bundles $WORKDIR/
sudo cp -r config $WORKDIR/

echo "Creating systemd service..."
sudo tee /etc/systemd/system/${SERVICE_NAME}.service > /dev/null <<EOF
[Unit]
Description=OKF Server
After=network.target

[Service]
ExecStart=${INSTALL_PATH}
WorkingDirectory=${WORKDIR}
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF

echo "Reloading systemd..."
sudo systemctl daemon-reload

echo "Enabling service..."
sudo systemctl enable ${SERVICE_NAME}

echo "Starting service..."
sudo systemctl start ${SERVICE_NAME}

echo "OKF server installed and running."
