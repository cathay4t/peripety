#!/bin/bash
sudo modprobe scsi_debug opts=2 every_nth=1
for x in /sys/block/*; do
    model=$(sed -e 's/ \+$//' $x/device/model)
    if [ "CHK$model" == "CHKscsi_debug" ];then
        disk=$(basename $x)
        break
    fi
done
if [ "CHK$disk" == "CHK" ];then
    echo "BUG: Failed to find scsi_debug disks"
    exit 1
fi
set -x
sudo dd if=/dev/$disk of=/dev/null
set +x
sleep 1
sudo modprobe -r scsi_debug
