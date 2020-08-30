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

APN="YOUR_APN"

while true
do

  cm radio off
  sleep 5

  # The apn seems to reset every once in a while
  cm data apn ${APN}

  cm radio on 
  sleep 5

  cm data connect &
  CM_PID=$!

  # Wait for 6 hours
  sleep 21600

  kill -9 ${CM_PID}

done
