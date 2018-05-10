#!/bin/bash

if [ "CHK$1" == "CHK" ];then
    exit 1
fi

wwid=`cat "/sys/block/$(basename $1)/device/wwid"| \
    sed -e 's/ \+$//' -e 's/ \+/-/g'`
shift

journalctl DEV_WWID="$wwid" + OWNERS_WWIDS="$wwid" $@
