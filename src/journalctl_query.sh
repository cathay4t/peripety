#!/bin/bash

FIELDS="RAW_MESSAGE,DEV_WWID,DEV_NAME,DEV_PATH,OWNERS_WWIDS,OWNERS_PATHS,"
FIELDS="${FIELDS},EVENT_TYPE,SUB_SYSTEM,"
# fs extensions.
FIELDS="${FIELDS},EXT_MOUNT_POINT"
# mpath extensions.
FIELDS="${FIELDS},EXT_DRIVER_NAME"
# mpath fc extensions
FIELDS="${FIELDS},EXT_TARGET_WWPN,EXT_HOST_WWPN,EXT_SPEED,EXT_PORT_STATE"
# mpath iscsi extensions
FIELDS="${FIELDS},EXT_ADDRESS,EXT_PORT,EXT_TPGT,EXT_TARGET_NAME,EXT_IFACE_NAME"
# scsi extensions
FIELDS="${FIELDS},EXT_SCSI_ID"

if [ "CHK$1" == "CHK" ];then
    exit 1
fi

wwid=`cat "/sys/block/$(basename $1)/device/wwid"| \
    sed -e 's/ \+$//' -e 's/ \+/-/g' -e 's/\(\\\0\)\+$//g'`
shift

journalctl --output-fields="$FIELDS" DEV_WWID="$wwid" + OWNERS_WWIDS="$wwid" $@
