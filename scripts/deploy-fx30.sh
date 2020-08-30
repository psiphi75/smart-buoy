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
FX30="root@192.168.2.2"

for ARG in $@; do

  case $ARG in

    "--aws") 
      UPLOAD_TO_AWS=1
      ;;

    "--force")
      SKIP_GIT_CHECK=1
      IS_FORCE="--force"
      ;;
    
    "--no-reboot")
      NO_REBOOT=1
      ;;

    *)
      DOMAIN=$ARG
      ;;

  esac

done


if [ -z ${DOMAIN} ]; then
  echo "Usage:"
  echo "  ./deploy-fx30.sh [domain.name.com] [--force] [--aws]"
  exit 1
fi

# 1. Build

./build-fx30.sh $DOMAIN $IS_FORCE

# 2. Copy or upload

if [[ $UPLOAD_TO_AWS != 1 ]]; then

  # The file
  FX30_FIRMWARE="/home/root/new-firmware.tar.gz"

  ssh ${FX30} "rm -rf /home/root/certs" # Clean up in advance
  scp ../target/fx30-build/fx30-firmware-*.tar.gz ${FX30}:${FX30_FIRMWARE}
  ssh ${FX30} "tar xf ${FX30_FIRMWARE} -C /"
  ssh ${FX30} "rm ${FX30_FIRMWARE}"

  if [[ $NO_REBOOT != 1 ]]; then
    echo "Rebooting"
    ssh ${FX30} "/sbin/reboot"
  else
    echo "Not rebooting"
  fi
  
  echo
  echo "SUCCESS"

else 

  ##
  ##
  ## Upload to S3
  ##
  ##

  AWS_S3_PATH="AWS_S3_PATH"
  PACKAGE_PATH=$(ls ../target/PATH/fx30-firmware-*.tar.gz)
  PACKAGE_FILENAME=$(basename ${PACKAGE_PATH})

  aws s3 cp ${PACKAGE_PATH} s3://${AWS_S3_PATH}/${PACKAGE_FILENAME} --acl public-read
  retval=$?

  case $retval in
      0)  echo "upload_firmware.sh:    Upload OK"
          echo "Send the following SMS to the FX30:"
          echo "   'UPGRADE {VERSION_CODE}', where the file is: fx30-firmware-{VERSION_CODE}.tar.gz"
          echo " file: ${PACKAGE_FILENAME}"
          ;;
      *) echo "upload_firmware.sh:    Upload to S3 FAILED!!"
          exit 1
	  ;;
  esac

fi

