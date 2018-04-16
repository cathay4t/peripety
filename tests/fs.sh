#!/bin/bash
modprobe scsi_debug dev_size_mb=500

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

dd if=/dev/urandom of=/tmp/haha count=400 bs=1M

# Write failure, but read will pass as file system will find another
# inode when sectore failure.
modprobe scsi_debug
dmsetup create bad_disk << EOF
  0 10000       linear /dev/sdb 0
  10000 1       error
  10001 1010000 linear /dev/sdb 10001
EOF

# Buffer I/O
dd if=/dev/urandom of=/dev/mapper/bad_disk bs=512 count=1 seek=10000

# Ext4
mkfs.ext4 /dev/mapper/bad_disk

mount /dev/mapper/bad_disk /mnt
cp -f /tmp/haha /mnt/
md5sum /tmp/haha /mnt/haha
umount /mnt

# xfs
mkfs.xfs -f /dev/mapper/bad_disk

mount /dev/mapper/bad_disk /mnt
cp -f /tmp/haha /mnt/
md5sum /tmp/haha /mnt/haha
umount /mnt

# Clean up
rm /tmp/haha
dmsetup remove bad_disk
modprobe -r scsi_debug
