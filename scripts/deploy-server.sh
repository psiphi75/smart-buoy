#!/bin/bash

###############################################################################
#
#   Smart-Buoy - connects marine sounds to the cloud.
#   Copyright (C) 2020  Simon M. Werner (Anemoi Robotics Ltd)
#
#   This program is free software: you can redistribute it and/or modify
#   it under the terms of the GNU General Public License as published by
#   the Free Software Foundation, either version 3 of the License, or
#   (at your option) any later version.
#
#   This program is distributed in the hope that it will be useful,
#   but WITHOUT ANY WARRANTY; without even the implied warranty of
#   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#   GNU General Public License for more details.
#
#   You should have received a copy of the GNU General Public License
#   along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
###############################################################################

set -e

if [ -z $1 ]; then
  echo "Usage:"
  echo "  ./deploy-server.sh [domain.name.com]"
  exit 1
fi
DOMAIN=$1


REMOTE=remote-server
STAGE=release

ROOT_DIR=../
if [ ! -d "${ROOT_DIR}"/target ]; then
  echo "'${ROOT_DIR}/target' is not a folder."
fi

#
# 1. Build
#

STAGE_ARG=""
if [ "release" == "${STAGE}" ]; then
  STAGE_ARG="--release"
fi
cargo clean --package buoy_code ${STAGE_FLAG}
DOMAIN=${DOMAIN} cargo build --bin server ${STAGE_ARG}

#
# 2. Deploy
#

ssh ${REMOTE} "sudo service buoy-server stop"     # Start it

scp -p ${ROOT_DIR}/target/${STAGE}/server ${REMOTE}:/home/ubuntu/buoy-server

ssh ${REMOTE} "mkdir -p /home/ubuntu/certs/${DOMAIN}"
scp -p ${ROOT_DIR}/certs/${DOMAIN}/server*         ${REMOTE}:/home/ubuntu/certs/${DOMAIN}/

ssh ${REMOTE} "sudo systemctl enable buoy-server"    # Enable on reboot
ssh ${REMOTE} "sudo service buoy-server restart"     # Start it
ssh ${REMOTE} "sudo service buoy-server status"      # Print out the details

echo
echo "SUCCESS"