#!/bin/sh
set -eu
exec /opt/ultimatepr-station/current/ultimatepr-station \
  --bind 0.0.0.0:8080 \
  --config /etc/ultimatepr/station.conf \
  --web-config /etc/ultimatepr/web.conf
