# Yuki OS - Hobby Operating System Written in Rust

## Features

### Current Features
- **Memory Management**
  - Dynamic memory manager for both kernel and userspace
  - Virtual memory support
- **Process Management**
  - Round-robin process scheduling
  - Ring 3 usermode support
  - Syscall interface
  - ELF file loader
- **Storage & Filesystems**
  - ATA HDD driver
  - FAT filesystem support
  - Virtual Filesystem (VFS) layer
  - Ramdisk support
- **Input/Output**
  - Keyboard and mouse drivers
  - Framebuffer support
  - Stdio files for applications

### In Progress
- Usermode window manager
- Rust std library support

### Future Plans
- TCP/IP networking support
- Multi-core support

## Building

### Prerequisites
- QEMU in path

### Build and run in QEMU
`cargo run`
