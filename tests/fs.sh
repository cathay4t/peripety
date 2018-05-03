#!/bin/bash
MNT_POINT=`mktemp -d`;
TMP_FILE=`mktemp`;

sudo modprobe scsi_debug dev_size_mb=500

for x in /sys/block/*; do
    if [ ! -e $x/device/model ];then
        continue;
    fi
    model=$(sed -e 's/ \+$//' $x/device/model)
    if [ "CHK$model" == "CHKscsi_debug" ];then
        disk=$(basename $x)
        break
    fi
done

# Write failure, but read will pass as file system will find another
# inode when sectore failure.
sudo dmsetup create bad_disk << EOF
  0 10000       linear /dev/$disk 0
  10000 1       error
  10001 1010000 linear /dev/$disk 10001
EOF

sudo dd if=/dev/urandom of=$TMP_FILE count=470 bs=1M

# Buffer I/O
sudo dd if=/dev/urandom of=/dev/mapper/bad_disk bs=512 count=1 seek=10000

# Ext4
sudo mkfs.ext4 -F /dev/mapper/bad_disk

sudo mount /dev/mapper/bad_disk $MNT_POINT
sudo cp -f $TMP_FILE $MNT_POINT/haha
sudo md5sum $TMP_FILE $MNT_POINT/haha
sudo umount $MNT_POINT

# xfs
sudo mkfs.xfs -f /dev/mapper/bad_disk

sudo mount /dev/mapper/bad_disk $MNT_POINT
sudo cp -f $TMP_FILE $MNT_POINT/haha
sudo md5sum $TMP_FILE $MNT_POINT/haha
sudo umount $MNT_POINT

# Clean up
sudo rm $TMP_FILE
sudo dmsetup remove bad_disk
sudo modprobe -r scsi_debug
