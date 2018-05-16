#!/bin/bash -x

sudo modprobe scsi_debug dev_size_mb=100 \
    vpd_use_hostno=0 add_host=8 max_luns=2
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
sudo multipath -ll

echo offline | sudo tee /sys/block/$disk/device/state
sleep 30
echo running | sudo tee /sys/block/$disk/device/state
if [ "CHK$1" != "CHK" ];then
    exit
fi
sleep 30
sudo systemctl stop multipathd
sudo multipath -F
sleep 5
sudo modprobe -r scsi_debug
