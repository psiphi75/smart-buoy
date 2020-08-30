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

##
##
## This file gets run on boot
##
##

source /etc/run.env
PATH=$PATH:/legato/systems/current/bin/

export ERR_LOG=/home/root/error.log
export STD_LOG=/dev/null

rm -f ${ERR_LOG}
rm -f /home/root/core-*   # Remove core dumps

##
## Power Management
##

# Disable ethernet
echo 0 > /sys/class/gpio/gpio55/value  2>> ${ERR_LOG}

# Disable the power LED
echo 0 > /sys/class/gpio/gpio49/value  2>> ${ERR_LOG}
echo 1 > /sys/class/gpio/gpio50/value  2>> ${ERR_LOG}
echo 1 > /sys/class/gpio/gpio51/value  2>> ${ERR_LOG}

##
## Navigation light
##

echo 0 > /sys/class/gpio/gpio56/value # Set the navigation light to off

##
## We don't need wifi
##

app stop wifiService

##
## Connect data
##

/home/root/data.sh    >>  ${STD_LOG} 2>> ${ERR_LOG} &

##
## Start the buoy
##

cd /home/root
./buoy   >>  ${STD_LOG} 2>> ${ERR_LOG}

##
## Clean up and reboot
##

echo "BUOY FAILED" >> ${ERR_LOG}
cp ${ERR_LOG}  ${ERR_LOG}.old
sleep 300
reboot
