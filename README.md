# Smart Buoy

Smart Buoy code developed as a proof of concept. This solution is built for the FX30.
This solution uses the FX30 and an Oceaninstruments hydrophone. The audio data from the
hydrophone is read in X3 compressed form and sent to a server over the
cellular network. It uses the QUIC (HTTP3) protocol for communication.
The server will decompress the X3 audio and create a .wav and a .png
spectrogram. A .json file will also be created, this contains details
about the incoming recording and the buoy status.

This repository contains all the code to orchestrate the above.

## The code structure

`/src/`

Rust code for the FX30 and the server.

`/src/bin/server`

The Rust server to listen to the FX30. It can listen
to multiple FX30s.

`/src/bin/buoy`

The code which runs on the FX30.

`/legato/SMS_Controller`

This is C code using the Legato framework for the FX30. The scripts can be found in
`fx30/home/root/sms_scripts`. This code will listen to incoming SMSs and do various actions:

- "REBOOT" - reboot the FX30.
- "ULPM N" - switch to ultra low power mode for N seconds.
- "UPGRADE" - connect to the server to download the latest version.

`/legato/TaringaComponent`

This will start the Rust code on boot.

## Connecting to the FX30 to a PC

Connect the power cable. Connect the USB of the PC to the micro-USB of the FX30.
The FX30 will create a network connection. On the PC you should see a new network
interface with an IP address. You can use the following command to ssh into the FX30.

```sh
ssh root@192.168.2.2

# no password is required
```

## FX30 Tips and Tricks

The [`cm` (Cellular Modem)](https://docs.legato.io/latest/toolsTarget_cm.html) tool helps create a connection.

Common commands are:

```sh
cm data info      # Get info
cm data connect   # Connect the device to the network (need to update iptables).
```

On Ubuntu machines, you made need to uninstall `modemmanager` to be able to ssh to the modem. And other
software needs to be installed

```sh
sudo apt purge modemmanager
sudo apt autoremove

# Install - required for serialport-rs
sudo apt install libudev-dev pkg-config

# For testing the hydrophone on a desktop machine you need to run the following
# Then you need to REBOOT!!
sudo usermod -a -G dialout $USER
sudo usermod -a -G tty $USER

```

## iptables

For something like `wget google.com` to work, you need to update the iptables.

```sh
# This will clear all INPUT filters.
iptables -F INPUT
```

## Low power configuration

```sh
# Disable ethernet
echo 0 > /sys/class/gpio/gpio55/value

# Disable the power LED
echo 0 > /sys/class/gpio/gpio49/value
echo 1 > /sys/class/gpio/gpio50/value
echo 1 > /sys/class/gpio/gpio51/value

# Disable the GPS: See: https://forum.sierrawireless.com/t/fx30-how-to-turn-off-gps/15961
#echo 0 > /sys/class/gpio/gpio52/value

# Disable the radio
cm radio off
```

## SMS commands

You can send an SMS to the device (you need the phone number). The SMS can run different commands.
Below are the commands that can be executed.

_Upgrade_:

```sh
UPGRADE [upgrade_version_code]
```

Example: "`UPGRADE 201906061858-d0d5a6`"

_Reboot the device_:

```sh
REBOOT
```

_Go into Ultra Low Power Mode_:

```sh
ULPM [time_seconds]
```

## Manuall running GPS

To manually run the GPS, ssh to the FX30 and run the following commands:

```sh
killall buoy    # This will kill the buoy process

gnss fix        # This will run `gnss enable` and `gnss start`
gnss get loc2d
```

The GPS commands can be found here: https://docs.legato.io/latest/toolsTarget_gnss.html

## Compiling with rust

Install linaro compiler collection to `/opt/gcc-linaro-arm-linux-gnueabihf`.

The use the following to compile:

```sh
CC=/opt/gcc-linaro-arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc cargo build --target=armv7-unknown-linux-musleabihf --release
```

then copy it to the FX30.

```sh
cd scripts
./deploy-fx30.sh
```

## Testing locally in a Linux box

```sh
# Get the server up and running
DOMAIN=localhost cargo run --bin server
```

```sh
# Get the server up and running
DOMAIN=localhost cargo run --bin buoy
```

## Upgrading firmware

Get the latest firmware from [here](https://source.sierrawireless.com/resources/airlink/software_downloads/fx30-firmware/fx30-firmware/).

It will download as `mcu-rmfw-boot-yocto-legato_wp85.cwe`.

The release notes explain the steps, but this will work too.

```sh
scp mcu-rmfw-boot-yocto-legato_wp85.cwe root@192.168.2.2:/home/root
ssh root@192.168.2.2

# now you're in the FX30

fwupdate download mcu-rmfw-boot-yocto-legato_wp85.cwe

# This will take a couple of minutes to be able to log back in.

```

## Installing Legato Development Environment

Based the isntructions on this [page](https://source.sierrawireless.com/resources/airprime/software/legato_application_development_kit_linux/).

### Steps

- Create an Ubuntu 16.04 Virtual Machine.

  - It will require around 12GB disk space.
  - Set up a 2nd Network Adapter as a "Bridged Adapter" with the FX30 network card as the interface.

- Run the following steps

```sh
wget https://downloads.sierrawireless.com/tools/leaf/leaf_latest.deb -O /tmp/leaf_latest.deb
sudo apt install /tmp/leaf_latest.deb

leaf search -t stable -t wp85     # This will display the latest stable releases
leaf setup wp85stable -p swi-wp85_2.0.5    # Choose the most recent stable release

```

- Install the Java dependancies
- Download and install Legato Dev Environment Release 15 (Legato 16.10.4), from [here](https://source.sierrawireless.com/resources/airprime/software/legato_history/):
- Compile the taringa-legato and deploy it to the FX30.


## License

    Smart-Buoy - connects marine sounds to the cloud.
    Copyright (C) 2020  Simon M. Werner (Anemoi Robotics Ltd)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

