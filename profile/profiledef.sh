#!/usr/bin/env bash

iso_name="blunux2"
iso_label="BLUNUX2_$(date +%Y%m)"
iso_publisher="blunux2 Project <https://blunux2.dev>"
iso_application="blunux2 Live/Install Medium"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=(
    'bios.syslinux.mbr'
    'bios.syslinux.eltorito'
    'uefi-ia32.grub.esp'
    'uefi-x64.grub.esp'
    'uefi-ia32.grub.eltorito'
    'uefi-x64.grub.eltorito'
)
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=(
    '-comp' 'zstd' '-Xcompression-level' '15'
    '-b' '1M'
)
file_permissions=(
    ["/etc/shadow"]="0:0:400"
    ["/usr/bin/startblunux"]="0:0:755"
    ["/usr/bin/blunux-wizard"]="0:0:755"
    ["/usr/bin/calamares-blunux"]="0:0:755"
    ["/usr/bin/blunux-toml2cal"]="0:0:755"
)
