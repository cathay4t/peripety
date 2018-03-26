#!/bin/bash -x

sudo modprobe scsi_debug dev_size_mb=500
sudo pvcreate /dev/sdb
sudo vgcreate vg /dev/sdb
sudo lvcreate -n ThinPoolLV -L 200M vg
sudo lvcreate -n ThinMetaLV -L 50M vg
sudo lvconvert \
    --type thin-pool --poolmetadata vg/ThinMetaLV vg/ThinPoolLV -f -y
sudo lvchange --errorwhenfull y vg/ThinPoolLV
sudo lvcreate -n ThinLV -V 1g --thinpool ThinPoolLV vg
sudo dd if=/dev/zero of=/dev/mapper/vg-ThinLV bs=1M count=201
if [ "CHK$1" != "CHK" ];then
    exit
fi
sudo lvremove vg/ThinLV -y
sudo vgchange -an
sudo vgremove vg -y
sudo modprobe -r dm-thin-pool
sleep 5
sudo modprobe -r scsi_debug
