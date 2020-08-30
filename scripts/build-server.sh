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


cat > /etc/systemd/system/buoy-server.service <<'__EOF'
[Unit]
Description=buoy-server
After=network.target

[Service]
Type=simple
Restart=always
RestartSec=1
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=buoy-server
User=ubuntu
WorkingDirectory=/home/ubuntu/
ExecStart=/usr/bin/env /home/ubuntu/buoy-server

[Install]
WantedBy=multi-user.target
__EOF

service buoy-server start
systemctl enable buoy-server
