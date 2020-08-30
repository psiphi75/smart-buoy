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
# Script to get the latest GPS values
#
# gnss doc: https://docs.legato.io/latest/toolsTarget_gnss.html
#
#

source /etc/run.env
PATH=$PATH:/legato/systems/current/bin/

# Timeout code
TIMEOUT=50
timeout_monitor() {
  sleep $TIMEOUT
  echo "GPS FAILED: timed out"
  kill $1
}
timeout_monitor $$ &
TIMEOUT_MONITOR_PID=$!


# Run a command
runcmd() {
  CMD=$1
  ENDS_WITH=$2
  
  # Run the command
  CMD_OUTPUT=$($CMD)

  case  $CMD_OUTPUT in *$ENDS_WITH)
    return
  esac

  echo "GPS FAILED: command '$CMD' returned '${CMD_OUTPUT}'"
  exit 1
}

# Expect:
#     GNSS was not enabled. Enabling it
#     TTFF not calculated (Position not fixed) BUSY
#     TTFF not calculated (Position not fixed) BUSY
#     TTFF not calculated (Position not fixed) BUSY
#     TTFF not calculated (Position not fixed) BUSY
#     TTFF start = 3188 msec
runcmd "gnss fix" "msec"

# Expect:
#     Latitude(positive->north) : -36.843292
#     Longitude(positive->east) : 174.756864
#     hAccuracy                 : 10.0m
echo $(gnss get loc2d)

# Expect:
#     Success!
runcmd "gnss stop" "Success!" > /dev/null

# Expect:
#     Success!
runcmd "gnss disable" "Success!" > /dev/null

kill $TIMEOUT_MONITOR_PID
