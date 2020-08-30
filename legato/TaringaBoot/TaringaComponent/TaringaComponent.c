///****************************************************************************
///
///  Smart-Buoy - connects marine sounds to the cloud.
///  Copyright (C) 2020  Simon M. Werner (Anemoi Robotics Ltd)
///
///  This program is free software: you can redistribute it and/or modify
///  it under the terms of the GNU General Public License as published by
///  the Free Software Foundation, either version 3 of the License, or
///  (at your option) any later version.
///
///  This program is distributed in the hope that it will be useful,
///  but WITHOUT ANY WARRANTY; without even the implied warranty of
///  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
///  GNU General Public License for more details.
///
///  You should have received a copy of the GNU General Public License
///  along with this program.  If not, see <https://www.gnu.org/licenses/>.
///
///****************************************************************************

#include <unistd.h>
#include "legato.h"

static void StartProcess(const char *wd, const char *cmd)
{
	int ch_result = chdir(wd);
	if (ch_result != 0)
	{
		LE_ERROR("Error changing directory to: %s", wd);
	}

	// This code is based on the following
	//    https://forum.mangoh.io/t/state-scrip/home/root/boot.sht-on-startup/1047/2

	char line[256];
	line[0] = '\0';
	LE_INFO("Running %s", cmd);
	FILE *fp = popen(cmd, "r");
	LE_ASSERT(fp != NULL);

	while (fgets(line, sizeof(line), fp) != NULL)
	{
		LE_INFO("script output: '%s'", line);
	}

	int result = pclose(fp);
	LE_FATAL_IF(!WIFEXITED(result), "Could not run boot script");
	const int exitCode = WEXITSTATUS(result);
	LE_FATAL_IF(exitCode != 0, "boot script failed with exit code %d", exitCode);
}

COMPONENT_INIT
{

	const char *WD = "/home/root";
	const char *CMD = "./boot.sh";

	StartProcess(WD, CMD);
}
