# Rust BCM2835(6/7) HWRNG Linux driver

The driver was implemented as a part of my Diploma Thesis in Computer Engineering.

The aim was to perform a basic evaluation of the advantages and drawbacks of using Rust in Linux kernel modules.

## Prerequisites

* Linux tree from [Rust-for-Linux team](https://github.com/rust-for-linux/linux)<sup>[1]</sup> along with its dependencies.

* This Pull Request: [rust: platform: add ioremap_resource and get_resource methods](https://github.com/Rust-for-Linux/linux/pull/682)

* If you don't intend to use the configuration file<sup>[2]</sup> provided inside `modified_linux_files` you must make sure that the following options are enabled in your custom `.config`:
    * `CONFIG_RUST`
    * `CONFIG_COMMON_CLK`
    * `CONFIG_RESET_CONTROLLER`
    * `CONFIG_HW_RANDOM`
    * `CONFIG_RANDOM_TRUST_CPU`

---

[1] Tested on commit 459035ab65.

[2] It is suited to (and tested in a) Raspberry Pi 3B with a 64bit OS. Because it's pretty minimal some basic -but unneeded- features are disabled e.g. sound support. Graphics and network are enabled.

## Building the driver as an Out-of-Tree module

Assuming that you are compiling in a x86_64 computer.

1. Copy the contents of `modified_linux_files` to `linux`.

    If you won't use a custom configuration file:

    1.a. Rename `config` to `.config`.

3. `cd` to `linux` and compile the kernel:

        $ make LLVM=1 ARCH=arm64

    It is also recommended to use the `-j` option.

4. Assuming that the directory tree is as follows:

    * working_directory
        * linux
        * driver

    `cd` to `driver` and compile the driver:

        $ make

    if the linux tree is in a different location then you must supply `make` with the tree's path.

        $ make KDIR=<path_to_linux_tree>

---

The aforementioned steps will only produce a working out-of-tree kernel module. If you want a working kernel you must also compile and install the in-tree kernel modules and if necessary, the dtb that corresponds to your platform.
