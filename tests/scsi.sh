#!/bin/bash
sudo modprobe scsi_debug opts=2 every_nth=1
sudo dd if=/dev/sdb of=/dev/null
sudo modprobe -r scsi_debug
