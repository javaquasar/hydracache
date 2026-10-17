#!/bin/sh
set -eu
[ "$(id -u)" -eq 1000 ]
[ "$(id -g)" -eq 1000 ]
umask 077
mktemp -d /tmp/hc074-start-material-20261017.XXXXXX
# End.
