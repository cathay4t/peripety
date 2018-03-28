#!/bin/bash -x

sudo modprobe scsi_debug dev_size_mb=100 \
    vpd_use_hostno=0 add_host=8 max_luns=2
sudo modprobe dm-multipath
sudo systemctl restart multipathd
sudo multipath -ll
echo offline | sudo tee /sys/block/sdp/device/state
sleep 30
echo running | sudo tee /sys/block/sdp/device/state
if [ "CHK$1" != "CHK" ];then
    exit
fi
sleep 30
sudo systemctl stop multipathd
sudo multipath -F
sleep 5
sudo modprobe -r scsi_debug
