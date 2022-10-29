# SPDX-License-Identifier: GPL-2.0

KDIR ?= ../linux

default:
	$(MAKE) -C $(KDIR) M=$$PWD LLVM=1 ARCH=arm64

clean:
	rm -f *.ko *.mod *.mod.c *.o Module.symvers modules.order .*cmd