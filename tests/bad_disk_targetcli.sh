#!/bin/bash

targetcli /backstores/block create bad_disk /dev/mapper/bad_disk
targetcli /iscsi create iqn.2003-01.org.linux-iscsi.org:iscsi-targetcli
targetcli /iscsi/iqn.2003-01.org.linux-iscsi.org:iscsi-targetcli/tpg1/luns \
    create /backstores/block/bad_disk
