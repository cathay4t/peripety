#!/bin/bash -x
MNT_POINT=`mktemp -d`;

sudo modprobe scsi_debug dev_size_mb=500 \
    vpd_use_hostno=0 add_host=8 max_luns=1

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

sudo tee /etc/multipath.conf  << EOL
defaults {
    user_friendly_names     yes
    max_fds 20000
    uxsock_timeout 10000
}

blacklist {
    device {
        vendor .*
        product .*
    }
}
blacklist_exceptions {
    device {
        vendor Linux
        product scsi_debug
    }
    device {
        vendor LIO-ORG
        product .*
    }
}
EOL

sudo modprobe dm-multipath
sudo systemctl restart multipathd
sudo multipath -r
sleep 5

MPATH_NAME=$(sudo multipath -l /dev/$disk \
    | perl -ne 'print $1 if /^(mpath[a-z]+)/')

sudo pvcreate /dev/mapper/$MPATH_NAME
sudo vgcreate vg /dev/mapper/$MPATH_NAME
sudo lvcreate -n lv -L 400M vg || exit 1
sudo mkfs.ext4 -F /dev/mapper/vg-lv
sudo mount /dev/mapper/vg-lv $MNT_POINT

if [ "CHK$1" != "CHK" ];then
    exit 1
fi

sudo umount $MNT_POINT
sudo lvremove vg/lv -y
sudo vgchange -an
sudo vgremove vg -y
sudo pvremove /dev/mapper/$MPATH_NAME -y
sleep 5
sudo multipath -F
sudo systemctl stop multipathd
sleep 5
sudo modprobe -r scsi_debug
