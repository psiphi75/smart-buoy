#!/bin/sh

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

# Script to upgrade to latest version of smart-buoy

source /etc/run.env
PATH=$PATH:/legato/systems/current/bin/

ERR_LOG=/home/root/error.log
STD_LOG=/home/root/stdlog.log  # FIXME: Change this to /dev/null for the future
SMS_USER="YOUR_PHONE_NUMBER"
CONN_WAIT_TIMEOUT_SEC=600

VERSION_CODE=$1
FILENAME="fx30-firmware-${VERSION_CODE}.tar.gz"
FWPATH="/home/root/${FILENAME}"
URL="http://YOUR_SERVER/YOUR_PATH/${FILENAME}"

# This function will return "yes" or "no"
net_connected() {
  echo $(cm data info | grep "Connected:" | awk '{print $2}')
}

reboot_after_timeout() {
  sleep  ${CONN_WAIT_TIMEOUT_SEC}

  if [ $(net_connected) = "no" ]; then
    echo "Timed out connecting to network"  >> ${ERR_LOG}
    cm sms send ${SMS_USER} "Unable to connect to network, rebooting"
    sleep 5
    reboot
  fi
}

#
# Validate input
#

if [ -z "${VERSION_CODE}" ]; then
  echo "No VERSION_CODE supplied"   >> ${ERR_LOG}
  cm sms send ${SMS_USER} "No VERSION_CODE supplied, rebooting"
  sleep 5
  reboot
fi
echo "Starting upgrade, using: ${URL}" >> ${STD_LOG}

# Stop the buoy process - this will disconnect the network
app stop TaringaBoot
echo "app stop TaringaBoot: DONE" >> ${STD_LOG}


  echo  "Connecting to network" >> ${STD_LOG}
  cm data connect ${CONN_WAIT_TIMEOUT_SEC} >>  ${STD_LOG} 2>> ${ERR_LOG} &
  CM_CONENCT_PID=$!

  $(reboot_after_timeout) &

  # Check the `cm connect` process exited okay
  while ps | grep " ${CM_CONENCT_PID} "
  do
    echo  "   ... waiting for network" >> ${STD_LOG}
    sleep 5

    IS_CONNECTED=$(net_connected)
    if [ ${IS_CONNECTED} = "yes" ]; then
      break
    fi
  done

  if [ ${IS_CONNECTED} != "yes" ]; then 
    wait ${CM_CONENCT_PID}
    if [ $? -ne 0 ]; then
      echo "Error with `cm data connect`"  >> ${ERR_LOG}
      cm sms send ${SMS_USER} "Failed to connect to network, rebooting"
      sleep 5
      reboot
    fi
  fi



echo  "Connected to network" >> ${STD_LOG}
sleep 5


#
# Download the firmware, save it and check it
#
echo "Starting download"
wget_output=$(wget -q "${URL}" --output-document "${FWPATH}")
if [ $? -ne 0 ]; then
  echo "Error with wget: '${wget_output}'"  >> ${ERR_LOG}
  cm sms send ${SMS_USER} "wget failed, rebooting"
  sleep 5
  reboot
fi
echo "Downloaded firmware: ${FWPATH}" >> ${STD_LOG}

#
# Extract the firmware
#

tar xf ${FWPATH} -C /
if [ $? -ne 0 ]; then
  echo "Error extracting firmware"  >> ${ERR_LOG}
  cm sms send ${SMS_USER} "Error extracting firmware, rebooting"
  sleep 5
  reboot
fi
rm ${FWPATH}
sync

# reboot takes a few seconds
echo "Rebooting" >> ${STD_LOG}
cm sms send ${SMS_USER} "Firmware upgrade success, rebooting"
sleep 5
reboot &

exit 0
