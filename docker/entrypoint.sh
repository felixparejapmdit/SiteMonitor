#!/bin/sh
set -eu
umask 077

# Synology creates bind-mounted directories as root on first deployment.
# Limit ownership changes to SiteMonitor's own data directories.
if [ "$(id -u)" = "0" ]; then
    mkdir -p /app/data/uploads /app/data/keys
    chown sitemonitor:sitemonitor /app/data /app/data/uploads /app/data/keys
    exec gosu sitemonitor "$@"
fi

exec "$@"
