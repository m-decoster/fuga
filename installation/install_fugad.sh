#!/usr/bin/env bash
set -e

BIN_DIR="/usr/local/bin"
SERVICE_FILE="/etc/systemd/system/fugad.service"

echo "[*] Checking installation prerequisites..."

# Exit if the script is not run as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# Exit if no binaries exist
if [ ! -f "target/release/fugad" ] || [ ! -f "target/release/fugactl" ]; then
    echo "Binaries not found. Please build the project first using 'cargo build --release'."
    exit 1
fi

echo "[*] Installing Fuga daemon..."

# Create system group/user if missing
if ! getent group fuga > /dev/null; then
    groupadd --system fuga
fi

if ! id fuga > /dev/null 2>&1; then
    useradd --system --no-create-home --gid fuga \
        --home-dir /var/lib/fuga \
        --shell /usr/sbin/nologin \
        fuga
fi

# Install binaries
install -Dm0755 target/release/fugad "$BIN_DIR/fugad"
install -Dm0755 target/release/fugactl "$BIN_DIR/fugactl"

# Create directories
install -d -m0750 -o fuga -g fuga /var/lib/fuga
install -d -m0750 -o fuga -g fuga /var/log/fuga

# Install systemd unit
install -Dm0644 packaging/fugad.service "$SERVICE_FILE"

# Reload systemd and enable service
systemctl daemon-reload
systemctl enable fugad

echo "[+] Installation complete."
echo "   Use 'systemctl start fugad' to launch the daemon."
