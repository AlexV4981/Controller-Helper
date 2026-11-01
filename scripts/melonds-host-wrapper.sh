#!/bin/sh
exec /usr/bin/flatpak run --command=env net.kuribo64.melonDS "$HOME/.var/app/net.kuribo64.melonDS/data/controller-studio-integration/launch" "$@"
