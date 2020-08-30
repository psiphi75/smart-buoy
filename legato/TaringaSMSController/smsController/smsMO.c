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
#include "interfaces.h"

le_result_t smsmo_SendMessage(
    const char *destinationPtr, ///< [IN] The destination number.
    const char *textPtr         ///< [IN] The SMS message text.
)
{
    le_result_t res;
    le_sms_MsgRef_t myMsg;

    myMsg = le_sms_Create();
    if (!myMsg)
    {
        LE_ERROR("SMS message creation has failed!");
        return LE_FAULT;
    }

    res = le_sms_SetDestination(myMsg, destinationPtr);
    if (res != LE_OK)
    {
        LE_ERROR("le_sms_SetDestination has failed (res.%d)!", res);
        return LE_FAULT;
    }

    res = le_sms_SetText(myMsg, textPtr);
    if (res != LE_OK)
    {
        LE_ERROR("le_sms_SetText has failed (res.%d)!", res);
        return LE_FAULT;
    }

    res = le_sms_Send(myMsg);
    if (res != LE_OK)
    {
        LE_ERROR("le_sms_Send has failed (res.%d)!", res);
        return LE_FAULT;
    }
    else
    {
        LE_INFO("\"%s\" has been successfully sent to %s.", textPtr, destinationPtr);
    }

    le_sms_Delete(myMsg);

    return LE_OK;
}
