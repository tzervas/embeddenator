#!/bin/bash
# =============================================================================
# Debian 13.2 VM Infrastructure Setup for Engram Testing
# Target: Linux (Debian-based) with NVIDIA RTX 5080 GPU
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VM_NAME="debian-13.2-engram"
VM_DIR="${HOME}/VMs/debian-13.2"
QCOW2_IMAGE="${VM_DIR}/${VM_NAME}.qcow2"
DISK_SIZE="50G"
RAM_SIZE="8192"  # MB
CPU_CORES="4"
DEBIAN_VERSION="13"
DEBIAN_CODENAME="trixie"

# Cloud image URL (Debian 13/Trixie cloud images)
CLOUD_IMAGE_URL="https://cloud.debian.org/images/cloud/${DEBIAN_CODENAME}/daily/latest/debian-${DEBIAN_VERSION}-generic-amd64-daily.qcow2"
CLOUD_IMAGE_FILE="${VM_DIR}/debian-${DEBIAN_VERSION}-cloud.qcow2"

# Mount point for QCOW2 access
MOUNT_POINT="/mnt/debian-vm"
NBD_DEVICE="/dev/nbd0"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# =============================================================================
# SECTION 1: Check Prerequisites and Existing VMs
# =============================================================================

check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing_deps=()
    
    # Check for required packages
    for cmd in qemu-system-x86_64 qemu-img virsh virt-install; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_warn "Missing dependencies: ${missing_deps[*]}"
        log_info "Installing QEMU/KVM packages..."
        sudo apt-get update
        sudo apt-get install -y \
            qemu-kvm \
            qemu-utils \
            libvirt-daemon-system \
            libvirt-clients \
            virtinst \
            virt-manager \
            bridge-utils \
            cloud-image-utils \
            genisoimage
    fi
    
    # Check if KVM is available
    if [ ! -e /dev/kvm ]; then
        log_error "KVM not available. Check BIOS virtualization settings."
        log_info "Run: sudo modprobe kvm && sudo modprobe kvm_intel (or kvm_amd)"
        return 1
    fi
    
    # Check if user is in libvirt/kvm groups
    if ! groups | grep -qE '(libvirt|kvm)'; then
        log_warn "User not in libvirt/kvm groups. Adding..."
        sudo usermod -aG libvirt,kvm "$USER"
        log_warn "Please log out and back in for group changes to take effect"
    fi
    
    # Start libvirtd if not running
    if ! systemctl is-active --quiet libvirtd; then
        log_info "Starting libvirtd service..."
        sudo systemctl start libvirtd
        sudo systemctl enable libvirtd
    fi
    
    log_success "Prerequisites check complete"
}

check_existing_vms() {
    log_info "Searching for existing Debian VMs..."
    
    local search_locations=(
        "/var/lib/libvirt/images"
        "${HOME}/VMs"
        "${HOME}/.local/share/libvirt/images"
        "${HOME}/Documents/VMs"
        "/opt/vms"
    )
    
    echo ""
    echo "=== Existing QCOW2 Images ==="
    for loc in "${search_locations[@]}"; do
        if [ -d "$loc" ]; then
            local found=$(find "$loc" -name "*.qcow2" -type f 2>/dev/null)
            if [ -n "$found" ]; then
                echo "Location: $loc"
                echo "$found" | while read -r img; do
                    local size=$(qemu-img info "$img" 2>/dev/null | grep "virtual size" | awk '{print $3, $4}')
                    echo "  - $(basename "$img") ($size)"
                done
            fi
        fi
    done
    
    echo ""
    echo "=== Libvirt Domains ==="
    if command -v virsh &> /dev/null; then
        virsh list --all 2>/dev/null || echo "  (no domains or libvirt not accessible)"
    fi
    
    echo ""
    echo "=== Target VM Status ==="
    if [ -f "$QCOW2_IMAGE" ]; then
        log_success "Target VM image exists: $QCOW2_IMAGE"
        qemu-img info "$QCOW2_IMAGE"
        return 0
    else
        log_warn "Target VM image does not exist: $QCOW2_IMAGE"
        return 1
    fi
}

# =============================================================================
# SECTION 2: Create Debian 13.2 QCOW2 Image
# =============================================================================

download_cloud_image() {
    log_info "Downloading Debian ${DEBIAN_VERSION} cloud image..."
    
    mkdir -p "$VM_DIR"
    
    if [ -f "$CLOUD_IMAGE_FILE" ]; then
        log_info "Cloud image already exists, checking integrity..."
        if qemu-img check "$CLOUD_IMAGE_FILE" &>/dev/null; then
            log_success "Existing cloud image is valid"
            return 0
        else
            log_warn "Existing image corrupted, re-downloading..."
            rm -f "$CLOUD_IMAGE_FILE"
        fi
    fi
    
    # Download with progress
    wget --progress=bar:force -O "$CLOUD_IMAGE_FILE" "$CLOUD_IMAGE_URL" || {
        log_error "Failed to download cloud image from: $CLOUD_IMAGE_URL"
        log_info "Alternative: Download manually from https://cloud.debian.org/images/cloud/"
        return 1
    }
    
    log_success "Cloud image downloaded successfully"
}

create_qcow2_from_cloud() {
    log_info "Creating QCOW2 disk image (${DISK_SIZE}) from cloud image..."
    
    mkdir -p "$VM_DIR"
    
    # Create a copy of the cloud image and resize it
    cp "$CLOUD_IMAGE_FILE" "$QCOW2_IMAGE"
    qemu-img resize "$QCOW2_IMAGE" "$DISK_SIZE"
    
    log_success "QCOW2 image created: $QCOW2_IMAGE"
    qemu-img info "$QCOW2_IMAGE"
}

create_cloud_init_config() {
    log_info "Creating cloud-init configuration..."
    
    local meta_data="${VM_DIR}/meta-data"
    local user_data="${VM_DIR}/user-data"
    local cloud_init_iso="${VM_DIR}/cloud-init.iso"
    
    # Meta-data
    cat > "$meta_data" << EOF
instance-id: ${VM_NAME}
local-hostname: ${VM_NAME}
EOF

    # User-data with engram testing setup
    cat > "$user_data" << 'EOF'
#cloud-config
hostname: debian-13-engram
manage_etc_hosts: true
users:
  - name: engram
    sudo: ALL=(ALL) NOPASSWD:ALL
    groups: sudo, docker
    shell: /bin/bash
    ssh_authorized_keys:
      - ssh-rsa AAAAB3NzaC1yc2E... # Add your SSH key here
    lock_passwd: false
    # Password: engram (change in production!)
    passwd: $6$rounds=4096$xyz$abcdef...

# Packages for engram testing
packages:
  - build-essential
  - curl
  - git
  - htop
  - vim
  - tmux
  - python3
  - python3-pip
  - python3-venv
  - pkg-config
  - libssl-dev
  - linux-headers-amd64
  - dkms
  - pciutils
  - lshw

# GPU passthrough preparation
runcmd:
  - |
    # Enable IOMMU in GRUB (requires reboot)
    if ! grep -q "intel_iommu=on" /etc/default/grub; then
      sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="/GRUB_CMDLINE_LINUX_DEFAULT="intel_iommu=on iommu=pt /' /etc/default/grub
      update-grub
    fi
  - |
    # Load VFIO modules
    echo "vfio" >> /etc/modules
    echo "vfio_iommu_type1" >> /etc/modules
    echo "vfio_pci" >> /etc/modules
    echo "vfio_virqfd" >> /etc/modules
  - systemctl enable ssh
  - systemctl start ssh

# Grow root partition to fill disk
growpart:
  mode: auto
  devices: ['/']
resize_rootfs: true

final_message: "Debian 13.2 Engram VM ready after $UPTIME seconds"
EOF

    # Generate cloud-init ISO
    genisoimage -output "$cloud_init_iso" \
        -volid cidata -joliet -rock \
        "$user_data" "$meta_data"
    
    log_success "Cloud-init ISO created: $cloud_init_iso"
}

create_vm_libvirt() {
    log_info "Creating libvirt VM domain..."
    
    local cloud_init_iso="${VM_DIR}/cloud-init.iso"
    
    # Check if VM already exists
    if virsh dominfo "$VM_NAME" &>/dev/null; then
        log_warn "VM '$VM_NAME' already exists"
        read -p "Destroy and recreate? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            virsh destroy "$VM_NAME" 2>/dev/null || true
            virsh undefine "$VM_NAME" --nvram 2>/dev/null || true
        else
            return 0
        fi
    fi
    
    virt-install \
        --name "$VM_NAME" \
        --ram "$RAM_SIZE" \
        --vcpus "$CPU_CORES" \
        --os-variant debian11 \
        --disk path="$QCOW2_IMAGE",format=qcow2,bus=virtio \
        --disk path="$cloud_init_iso",device=cdrom \
        --network network=default,model=virtio \
        --graphics vnc,listen=0.0.0.0 \
        --console pty,target_type=serial \
        --boot uefi \
        --noautoconsole \
        --import
    
    log_success "VM '$VM_NAME' created and started"
    log_info "Connect with: virsh console $VM_NAME"
    log_info "Or VNC: virt-viewer $VM_NAME"
}

# =============================================================================
# SECTION 3: Mount QCOW2 for Direct Filesystem Access
# =============================================================================

load_nbd_module() {
    if ! lsmod | grep -q "^nbd"; then
        log_info "Loading NBD kernel module..."
        sudo modprobe nbd max_part=8
    fi
}

mount_qcow2() {
    local image="${1:-$QCOW2_IMAGE}"
    
    if [ ! -f "$image" ]; then
        log_error "QCOW2 image not found: $image"
        return 1
    fi
    
    load_nbd_module
    
    # Check if NBD device is already in use
    if [ -e "${NBD_DEVICE}p1" ] || lsblk "$NBD_DEVICE" &>/dev/null 2>&1; then
        log_warn "NBD device $NBD_DEVICE appears to be in use"
        read -p "Disconnect and remount? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            unmount_qcow2
        else
            return 1
        fi
    fi
    
    log_info "Connecting QCOW2 to NBD device..."
    sudo qemu-nbd --connect="$NBD_DEVICE" "$image"
    
    # Wait for partitions to appear
    sleep 2
    sudo partprobe "$NBD_DEVICE"
    sleep 1
    
    # List partitions
    log_info "Available partitions:"
    lsblk "$NBD_DEVICE"
    
    # Create mount point
    sudo mkdir -p "$MOUNT_POINT"
    
    # Try to mount the main partition (usually p1 or p2)
    local mounted=false
    for part in "${NBD_DEVICE}p2" "${NBD_DEVICE}p1" "$NBD_DEVICE"; do
        if [ -e "$part" ]; then
            log_info "Attempting to mount $part..."
            if sudo mount "$part" "$MOUNT_POINT" 2>/dev/null; then
                mounted=true
                break
            fi
        fi
    done
    
    if [ "$mounted" = true ]; then
        log_success "QCOW2 mounted at: $MOUNT_POINT"
        echo ""
        echo "=== Mounted Filesystem ==="
        df -h "$MOUNT_POINT"
        echo ""
        ls -la "$MOUNT_POINT"
    else
        log_error "Failed to mount any partition"
        sudo qemu-nbd --disconnect "$NBD_DEVICE"
        return 1
    fi
}

unmount_qcow2() {
    log_info "Unmounting QCOW2..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        sudo umount "$MOUNT_POINT"
        log_success "Unmounted $MOUNT_POINT"
    fi
    
    # Disconnect NBD
    if lsblk "$NBD_DEVICE" &>/dev/null 2>&1; then
        sudo qemu-nbd --disconnect "$NBD_DEVICE"
        log_success "Disconnected $NBD_DEVICE"
    fi
    
    # Clean up mount point
    if [ -d "$MOUNT_POINT" ] && [ -z "$(ls -A "$MOUNT_POINT" 2>/dev/null)" ]; then
        sudo rmdir "$MOUNT_POINT" 2>/dev/null || true
    fi
}

# =============================================================================
# SECTION 4: GPU Passthrough Verification (NVIDIA RTX 5080)
# =============================================================================

check_iommu_status() {
    log_info "Checking IOMMU status..."
    
    echo ""
    echo "=== IOMMU Kernel Parameters ==="
    cat /proc/cmdline | tr ' ' '\n' | grep -E "(iommu|vfio)" || echo "(no IOMMU params found)"
    
    echo ""
    echo "=== IOMMU Groups ==="
    if [ -d /sys/kernel/iommu_groups ]; then
        local group_count=$(ls /sys/kernel/iommu_groups/ | wc -l)
        log_success "IOMMU is enabled ($group_count groups found)"
        
        # List groups with NVIDIA devices
        echo ""
        echo "=== NVIDIA Devices in IOMMU Groups ==="
        for group in /sys/kernel/iommu_groups/*/devices/*; do
            if [ -e "$group" ]; then
                local pci_id=$(basename "$group")
                local vendor=$(cat "/sys/bus/pci/devices/$pci_id/vendor" 2>/dev/null)
                if [ "$vendor" = "0x10de" ]; then  # NVIDIA vendor ID
                    local group_num=$(echo "$group" | grep -oP 'iommu_groups/\K[0-9]+')
                    local device_name=$(lspci -s "$pci_id" 2>/dev/null)
                    echo "  Group $group_num: $device_name"
                fi
            fi
        done
    else
        log_error "IOMMU not enabled! Add these kernel parameters:"
        echo "  Intel CPU: intel_iommu=on iommu=pt"
        echo "  AMD CPU:   amd_iommu=on iommu=pt"
        return 1
    fi
}

check_vfio_status() {
    log_info "Checking VFIO configuration..."
    
    echo ""
    echo "=== VFIO Modules ==="
    for mod in vfio vfio_iommu_type1 vfio_pci vfio_virqfd; do
        if lsmod | grep -q "^$mod"; then
            echo "  [✓] $mod loaded"
        else
            echo "  [✗] $mod NOT loaded"
        fi
    done
    
    echo ""
    echo "=== VFIO Configuration Files ==="
    
    # Check /etc/modules
    if [ -f /etc/modules ]; then
        echo "In /etc/modules:"
        grep -E "vfio" /etc/modules 2>/dev/null || echo "  (no VFIO entries)"
    fi
    
    # Check modprobe.d
    echo ""
    echo "In /etc/modprobe.d/:"
    for conf in /etc/modprobe.d/*vfio*.conf /etc/modprobe.d/*gpu*.conf; do
        if [ -f "$conf" ]; then
            echo "  $conf:"
            cat "$conf" | sed 's/^/    /'
        fi
    done 2>/dev/null || echo "  (no VFIO modprobe configs)"
}

check_nvidia_gpu() {
    log_info "Checking NVIDIA GPU status..."
    
    echo ""
    echo "=== NVIDIA GPUs Detected ==="
    lspci -nn | grep -i nvidia
    
    echo ""
    echo "=== GPU Details ==="
    for gpu in $(lspci -d 10de: -n | awk '{print $1}'); do
        echo "PCI Address: $gpu"
        lspci -v -s "$gpu" | head -20
        
        # Get IOMMU group
        local iommu_group=$(readlink /sys/bus/pci/devices/0000:$gpu/iommu_group 2>/dev/null | xargs basename 2>/dev/null)
        if [ -n "$iommu_group" ]; then
            echo "IOMMU Group: $iommu_group"
            echo "Other devices in group:"
            ls /sys/kernel/iommu_groups/$iommu_group/devices/ 2>/dev/null | while read dev; do
                echo "  - $(lspci -s ${dev#0000:} 2>/dev/null || echo $dev)"
            done
        fi
        echo ""
    done
    
    echo ""
    echo "=== Current GPU Driver Binding ==="
    for gpu in $(lspci -d 10de: -n | awk '{print $1}'); do
        local driver=$(readlink /sys/bus/pci/devices/0000:$gpu/driver 2>/dev/null | xargs basename 2>/dev/null)
        echo "  $gpu: ${driver:-no driver}"
    done
}

check_gpu_passthrough_readiness() {
    log_info "GPU Passthrough Readiness Check..."
    
    local issues=()
    local warnings=()
    
    # Check IOMMU
    if [ ! -d /sys/kernel/iommu_groups ] || [ -z "$(ls /sys/kernel/iommu_groups/)" ]; then
        issues+=("IOMMU not enabled")
    fi
    
    # Check VFIO modules
    for mod in vfio vfio_pci vfio_iommu_type1; do
        if ! lsmod | grep -q "^$mod"; then
            warnings+=("Module $mod not loaded")
        fi
    done
    
    # Check if GPU is bound to vfio-pci
    local nvidia_count=$(lspci -d 10de: -n | wc -l)
    if [ "$nvidia_count" -eq 0 ]; then
        issues+=("No NVIDIA GPU found")
    else
        local vfio_bound=$(for gpu in $(lspci -d 10de: -n | awk '{print $1}'); do
            readlink /sys/bus/pci/devices/0000:$gpu/driver 2>/dev/null | grep -q vfio && echo 1
        done | wc -l)
        
        if [ "$vfio_bound" -eq 0 ]; then
            warnings+=("No GPU bound to vfio-pci (needed for passthrough)")
        fi
    fi
    
    echo ""
    echo "=== Passthrough Readiness Summary ==="
    
    if [ ${#issues[@]} -eq 0 ] && [ ${#warnings[@]} -eq 0 ]; then
        log_success "System appears ready for GPU passthrough!"
    else
        if [ ${#issues[@]} -gt 0 ]; then
            log_error "Critical issues:"
            for issue in "${issues[@]}"; do
                echo "  - $issue"
            done
        fi
        if [ ${#warnings[@]} -gt 0 ]; then
            log_warn "Warnings:"
            for warning in "${warnings[@]}"; do
                echo "  - $warning"
            done
        fi
    fi
}

setup_vfio_for_gpu() {
    log_info "Setting up VFIO for GPU passthrough..."
    
    # Get NVIDIA GPU PCI IDs
    local gpu_ids=$(lspci -nn -d 10de: | grep -oP '\[\K[0-9a-f]{4}:[0-9a-f]{4}(?=\])' | sort -u | tr '\n' ',' | sed 's/,$//')
    
    if [ -z "$gpu_ids" ]; then
        log_error "No NVIDIA GPU found"
        return 1
    fi
    
    log_info "Found NVIDIA device IDs: $gpu_ids"
    
    # Create VFIO configuration
    cat << EOF | sudo tee /etc/modprobe.d/vfio.conf
# VFIO configuration for GPU passthrough
# NVIDIA RTX 5080 and associated devices
options vfio-pci ids=$gpu_ids
softdep nvidia pre: vfio-pci
softdep nouveau pre: vfio-pci
EOF

    # Ensure VFIO modules load early
    cat << EOF | sudo tee /etc/modules-load.d/vfio.conf
vfio
vfio_iommu_type1
vfio_pci
vfio_virqfd
EOF

    # Update GRUB for IOMMU
    if ! grep -q "intel_iommu=on\|amd_iommu=on" /etc/default/grub; then
        log_info "Updating GRUB for IOMMU..."
        
        # Detect CPU vendor
        local cpu_vendor=$(grep -m1 vendor_id /proc/cpuinfo | awk '{print $3}')
        local iommu_param=""
        
        if [ "$cpu_vendor" = "GenuineIntel" ]; then
            iommu_param="intel_iommu=on"
        else
            iommu_param="amd_iommu=on"
        fi
        
        sudo sed -i "s/GRUB_CMDLINE_LINUX_DEFAULT=\"/GRUB_CMDLINE_LINUX_DEFAULT=\"$iommu_param iommu=pt /" /etc/default/grub
        sudo update-grub
        
        log_warn "GRUB updated. Reboot required for IOMMU to take effect."
    fi
    
    # Update initramfs
    sudo update-initramfs -u
    
    log_success "VFIO configuration complete"
    log_warn "Reboot required to apply changes"
}

# =============================================================================
# SECTION 5: Quick Commands
# =============================================================================

print_quick_commands() {
    cat << 'EOF'

=============================================================================
                    QUICK REFERENCE COMMANDS
=============================================================================

# --- VM Management ---
virsh list --all                          # List all VMs
virsh start debian-13.2-engram            # Start VM
virsh shutdown debian-13.2-engram         # Graceful shutdown
virsh destroy debian-13.2-engram          # Force stop
virsh console debian-13.2-engram          # Serial console

# --- Mount QCOW2 Directly ---
sudo modprobe nbd max_part=8
sudo qemu-nbd --connect=/dev/nbd0 ~/VMs/debian-13.2/debian-13.2-engram.qcow2
sudo mount /dev/nbd0p2 /mnt/debian-vm     # or p1 depending on partition layout
# ... do work ...
sudo umount /mnt/debian-vm
sudo qemu-nbd --disconnect /dev/nbd0

# --- GPU Passthrough Check ---
lspci -nn | grep -i nvidia                # Find NVIDIA GPUs
ls /sys/kernel/iommu_groups/              # Check IOMMU groups
lsmod | grep vfio                         # Check VFIO modules
dmesg | grep -i iommu                     # IOMMU kernel messages

# --- Add GPU to VM (after VFIO setup) ---
# Find GPU PCI address (e.g., 01:00.0)
virsh nodedev-list --cap pci | grep nvidia
virsh nodedev-dumpxml pci_0000_01_00_0    # Get XML for attachment
# Edit VM XML to add hostdev for GPU

=============================================================================
EOF
}

# =============================================================================
# Main Menu
# =============================================================================

show_menu() {
    echo ""
    echo "==========================================="
    echo "  Debian 13.2 VM Infrastructure Setup"
    echo "==========================================="
    echo ""
    echo "1) Check prerequisites & existing VMs"
    echo "2) Create new Debian 13.2 VM (cloud image)"
    echo "3) Mount QCOW2 for direct access"
    echo "4) Unmount QCOW2"
    echo "5) Check GPU passthrough status"
    echo "6) Setup VFIO for GPU passthrough"
    echo "7) Print quick reference commands"
    echo "8) Full setup (1 + 2 + 5)"
    echo "9) Exit"
    echo ""
    read -p "Select option [1-9]: " choice
    
    case $choice in
        1)
            check_prerequisites
            check_existing_vms
            ;;
        2)
            check_prerequisites
            download_cloud_image
            create_qcow2_from_cloud
            create_cloud_init_config
            create_vm_libvirt
            ;;
        3)
            mount_qcow2
            ;;
        4)
            unmount_qcow2
            ;;
        5)
            check_iommu_status
            check_vfio_status
            check_nvidia_gpu
            check_gpu_passthrough_readiness
            ;;
        6)
            setup_vfio_for_gpu
            ;;
        7)
            print_quick_commands
            ;;
        8)
            check_prerequisites
            check_existing_vms || true
            download_cloud_image
            create_qcow2_from_cloud
            create_cloud_init_config
            create_vm_libvirt
            check_iommu_status
            check_vfio_status
            check_nvidia_gpu
            check_gpu_passthrough_readiness
            ;;
        9)
            exit 0
            ;;
        *)
            log_error "Invalid option"
            ;;
    esac
    
    show_menu
}

# =============================================================================
# Entry Point
# =============================================================================

# Allow running specific functions from command line
if [ $# -gt 0 ]; then
    case "$1" in
        --check)
            check_prerequisites
            check_existing_vms
            ;;
        --create)
            check_prerequisites
            download_cloud_image
            create_qcow2_from_cloud
            create_cloud_init_config
            create_vm_libvirt
            ;;
        --mount)
            mount_qcow2 "${2:-}"
            ;;
        --unmount)
            unmount_qcow2
            ;;
        --gpu-check)
            check_iommu_status
            check_vfio_status
            check_nvidia_gpu
            check_gpu_passthrough_readiness
            ;;
        --gpu-setup)
            setup_vfio_for_gpu
            ;;
        --help)
            echo "Usage: $0 [OPTION]"
            echo ""
            echo "Options:"
            echo "  --check       Check prerequisites and existing VMs"
            echo "  --create      Create new Debian 13.2 VM"
            echo "  --mount [img] Mount QCOW2 for direct access"
            echo "  --unmount     Unmount QCOW2"
            echo "  --gpu-check   Check GPU passthrough status"
            echo "  --gpu-setup   Setup VFIO for GPU passthrough"
            echo "  --help        Show this help"
            echo ""
            echo "Without options, interactive menu is shown."
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
else
    show_menu
fi
