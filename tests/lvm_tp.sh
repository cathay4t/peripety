#!/bin/bash -x
MNT_POINT=`mktemp -d`

sudo modprobe scsi_debug dev_size_mb=500
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

sudo pvcreate /dev/$disk
sudo vgcreate vg /dev/$disk
sudo lvcreate -n ThinPoolLV -L 200M vg
sudo lvcreate -n ThinMetaLV -L 50M vg
sudo lvconvert \
    --type thin-pool --poolmetadata vg/ThinMetaLV vg/ThinPoolLV -f -y
sudo lvchange --errorwhenfull y vg/ThinPoolLV
sudo lvcreate -n ThinLV -V 1g --thinpool ThinPoolLV vg
sudo mkfs.xfs -f /dev/mapper/vg-ThinLV
sudo mount /dev/mapper/vg-ThinLV $MNT_POINT
sudo dd if=/dev/zero of=$MNT_POINT/haha bs=1M count=201
if [ "CHK$1" != "CHK" ];then
    exit
fi
sleep 5
sudo umount $MNT_POINT
sudo lvremove vg/ThinLV -y
sudo vgchange -an
sudo vgremove vg -y
sudo modprobe -r dm-thin-pool
sleep 5
sudo modprobe -r scsi_debug
