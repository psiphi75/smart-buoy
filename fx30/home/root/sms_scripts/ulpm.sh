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

#
# Ultra low power mode
#

ULPM_SECONDS=$1

source /etc/run.env
PATH=$PATH:/legato/systems/current/bin/

# This command blocks the shutdown
# FIXME: devMode needs to be removed entirely. See: https://docs.legato.io/latest/basicTargetDevMode.html
app stop devMode

# Kill the data connection
killall cm
sleep 2

# Turn the radio off
cm radio off
sleep 2

pmtool bootOn timer $ULPM_SECONDS
pmtool shutdown &     # Fork to background such that service worker can finish
