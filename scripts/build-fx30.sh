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

BUILD_DIR=../target/fx30-build
BUILD_FILES=${BUILD_DIR}/root
STAGE="release"   # debug or release

##
##
## Ensure we are ready to build
##
##

if [[ $2 != "--force" ]]; then

    #
    # Validate changes are all committed
    #

    if [[ $(git status --porcelain) ]]; then
        echo "ERROR: There are uncommited changes.  Either stash them or commit them."
        exit 1
    fi

else

    echo "Forcing firmware generation"

fi

##
##
## Validate input
##
##

if [ -z $1 ]; then
  echo "Usage:"
  echo "  ./build-fx30.sh [domain.name.com]"
  exit 1
fi
DOMAIN=$1

CERT_DIR=../certs/${DOMAIN}
if [ ! -d ${CERT_DIR} ]; then
  echo "Certs directory missing: ${CERT_DIR}"
  exit 1
fi

##
##
## Build Binary
##
##

echo "Building for ${DOMAIN}"

ARCH_TARGET="armv7-unknown-linux-musleabihf"
LEGATO_CC="/opt/gcc-linaro-arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc"
if [[ ${STAGE} == "release" ]]; then
  STAGE_FLAG="--release"
fi

# compile
cargo clean --package gift_code ${STAGE_FLAG}
DOMAIN=${DOMAIN} CC=$LEGATO_CC cargo build --target=${ARCH_TARGET} --bin buoy ${STAGE_FLAG} --features "fx30"

if [ $? -ne 0 ]; then
  echo "Error with cargo build"
  exit 1
fi

##
##
## Build package
##
##

# Create a new version
function fn_create_new_version() {

    # Has of last commit
    BUILD_HASH=$(git rev-parse --short HEAD)

    VER_MAJOR=0
    VER_MINOR=$(git rev-list --count HEAD) # Number of commits
    VERSION=${VER_MAJOR}.${VER_MINOR}-${BUILD_HASH}

    echo $VERSION
}

FX30_ROOT=../fx30
UUID=$(dbus-uuidgen)
UUID=${UUID:0:6}
DATETIME=$(date '+%Y%m%d%H%M')

VERSION_CODE="${DATETIME}-${UUID}"

PACKAGE_FILENAME="fx30-firmware-${VERSION_CODE}.tar.gz"
PACKAGE_PATH="${BUILD_DIR}/${PACKAGE_FILENAME}"

if [ -z ${BUILD_FILES} ]; then
  echo "No ${BUILD_FILES}"
  exit 1
fi
rm -rf ${BUILD_DIR}/fx30*.tar.gz
rm -rf ${BUILD_DIR}/root

mkdir -p ${BUILD_FILES}

# Most files
cp -a ${FX30_ROOT}/*  ${BUILD_FILES}

# cert stuff
mkdir -p ${BUILD_FILES}/home/root/certs/${DOMAIN}
cp -a ${CERT_DIR}/ca.der ${BUILD_FILES}/home/root/certs/${DOMAIN}/

# the exe
cp ../target/${ARCH_TARGET}/${STAGE}/buoy ${BUILD_FILES}/home/root/buoy

# Let's create the tarball
tar czf ${PACKAGE_PATH} -C ${BUILD_FILES} .

echo "Created package: ${PACKAGE_PATH}"

