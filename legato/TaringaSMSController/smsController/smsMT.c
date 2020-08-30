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

#include "legato.h"

#include "../smsController/smsSample.h"
#include "interfaces.h"

#define MESSAGE_UPGRADE_SUCCESS "Upgrade started"
#define MESSAGE_REBOOT_SUCCESS "Rebooting FX30"
#define MESSAGE_ULPM_SUCCESS "Going into ULPM for %d seconds"
#define MESSAGE_FAIL "Unknown command '%s'"

static le_sms_RxMessageHandlerRef_t RxHdlrRef;
static le_sms_FullStorageEventHandlerRef_t FullStorageHdlrRef;

static void StartProcess(const char *wd, const char *cmd)
{
    int ch_result = chdir(wd);
    if (ch_result != 0)
    {
        LE_ERROR("Error changing directory to: %s", wd);
    }

    LE_INFO("Running script %s", cmd);
    int ret = system(cmd);
    if (ret != 0)
    {
        LE_ERROR("The script exited with an error");
    }
}

static void RxMessageHandler(
    le_sms_MsgRef_t msgRef,
    void *contextPtr)
{
    le_result_t res;
    char tel[LE_MDMDEFS_PHONE_NUM_MAX_BYTES];
    char timestamp[LE_SMS_TIMESTAMP_MAX_BYTES] = {0};
    char text[LE_SMS_TEXT_MAX_BYTES] = {0};
    char textReturn[LE_SMS_TEXT_MAX_BYTES] = {0};
    int ulpm_num_seconds = 0;
    char upgrade_version_code[256];
    upgrade_version_code[0] = '\0';
    bool do_reboot = false;

    LE_INFO("A New SMS message is received with ref.%p", msgRef);

    if (le_sms_GetFormat(msgRef) == LE_SMS_FORMAT_TEXT)
    {
        res = le_sms_GetSenderTel(msgRef, tel, sizeof(tel));
        if (res != LE_OK)
        {
            LE_ERROR("le_sms_GetSenderTel has failed (res.%d)!", res);
        }
        else
        {
            LE_INFO("Message is received from %s.", tel);
        }

        res = le_sms_GetTimeStamp(msgRef, timestamp, sizeof(timestamp));
        if (res != LE_OK)
        {
            LE_ERROR("le_sms_GetTimeStamp has failed (res.%d)!", res);
        }
        else
        {
            LE_INFO("Message timestamp is %s.", timestamp);
        }

        res = le_sms_GetText(msgRef, text, sizeof(text));
        if (res != LE_OK)
        {
            LE_ERROR("le_sms_GetText has failed (res.%d)!", res);
        }
        else
        {
            LE_INFO("Message content: \"%s\"", text);

            // Start upgrade
            if (sscanf(text, "UPGRADE %s", upgrade_version_code) == 1)
            {
                snprintf(textReturn, sizeof(textReturn), MESSAGE_UPGRADE_SUCCESS);
            }

            // Reboot the device
            else if (strcmp(text, "REBOOT") == 0)
            {
                snprintf(textReturn, sizeof(textReturn), MESSAGE_REBOOT_SUCCESS);
                do_reboot = true;
            }

            // Go into Ultra Low Power Mode
            else if (sscanf(text, "ULPM %d", &ulpm_num_seconds) == 1)
            {
                snprintf(textReturn, sizeof(textReturn), MESSAGE_ULPM_SUCCESS, ulpm_num_seconds);
            }

            // Otherwise I don't know
            else
            {
                snprintf(textReturn, sizeof(textReturn), MESSAGE_FAIL, text);
            }
        }

        // Return a message to sender with phone number include (see smsMO.c file)
        res = smsmo_SendMessage(tel, textReturn);
        if (res != LE_OK)
        {
            LE_ERROR("smsmo_SendMessage has failed (res.%d)!", res);
        }
        else
        {
            LE_INFO("The message has been successfully sent.");
        }

        res = le_sms_DeleteFromStorage(msgRef);
        if (res != LE_OK)
        {
            LE_ERROR("le_sms_DeleteFromStorage has failed (res.%d)!", res);
        }
        else
        {
            LE_INFO("the message has been successfully deleted from storage.");
        }
    }
    else
    {
        LE_WARN("Warning! I read only Text messages!");
    }

    le_sms_Delete(msgRef);

    //
    // Once the reply SMS messages have been sent, we do the actions
    //
    if (strlen(upgrade_version_code) > 0)
    {
        char upgrade_cmd[256];
        LE_INFO("Running upgrade: %s", upgrade_version_code);
        sprintf(upgrade_cmd, "./upgrade.sh %s", upgrade_version_code);
        StartProcess("/home/root/sms_scripts", upgrade_cmd);
    }

    if (do_reboot)
    {
        StartProcess("/home/root", "/sbin/reboot");
    }

    if (ulpm_num_seconds > 0)
    {
        char ulpm_cmd[30];
        sprintf(ulpm_cmd, "./ulpm.sh %d", ulpm_num_seconds);
        StartProcess("/home/root/sms_scripts", ulpm_cmd);
    }
}

static void StorageMessageHandler(
    le_sms_Storage_t storage,
    void *contextPtr)
{
    LE_INFO("A Full storage SMS message is received. Type of full storage %d", storage);
}

le_result_t smsmt_Receiver(
    void)
{
    RxHdlrRef = le_sms_AddRxMessageHandler(RxMessageHandler, NULL);
    if (!RxHdlrRef)
    {
        LE_ERROR("le_sms_AddRxMessageHandler has failed!");
        return LE_FAULT;
    }
    else
    {
        return LE_OK;
    }
}

le_result_t smsmt_MonitorStorage(
    void)
{
    FullStorageHdlrRef = le_sms_AddFullStorageEventHandler(StorageMessageHandler, NULL);
    if (!FullStorageHdlrRef)
    {
        LE_ERROR("le_sms_AddFullStorageEventHandler has failed!");
        return LE_FAULT;
    }
    else
    {
        return LE_OK;
    }
}

void smsmt_HandlerRemover(void)
{
    le_sms_RemoveRxMessageHandler(RxHdlrRef);
}

void smsmt_StorageHandlerRemover(void)
{
    le_sms_RemoveFullStorageEventHandler(FullStorageHdlrRef);
}
