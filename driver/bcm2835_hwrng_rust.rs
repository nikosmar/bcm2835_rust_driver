// SPDX-License-Identifier: GPL-2.0

//! Driver for the Broadcom 2835 Hardware Random Number Generator.
//!
//! Based on the C driver written by Broadcom and Lubomir Rintel.

use core::hint::spin_loop;
use core::mem::size_of;
use kernel::{
    clk,
    device,
    device::RawDevice,
    hwrng,
    io_mem::IoMem,
    module_platform_driver,
    of,
    platform,
    prelude::*,
    reset,
    sync::{RawSpinLock, Ref, RefBorrow},
};

const RNG_CTRL: usize = 0x0;
const RNG_STATUS: usize = 0x4;
const RNG_DATA: usize = 0x8;
const RNG_INT_MASK: usize = 0x10;
// Devices that mask interrupts need 4 additional bytes (0x14 in total)
const RNG_REG_SIZE: usize = 0x10;
// enable rng
const RNG_ENABLE: u32 = 0x1;
const RNG_DISABLE: u32 = 0x0;
// discard the initial numbers until enough entropy is gathered
const RNG_WARMUP_COUNT: u32 = 0x40000;
const RNG_INT_OFF: u32 = 0x1;

struct BCM2835RNGDev;

struct BCM2835Resources {
    base: IoMem<RNG_REG_SIZE>,
}

struct BCM2835General {
    _dev: device::Device,
    clk: RawSpinLock<Option<clk::EnabledClk>>,
    reset: RawSpinLock<Option<reset::Reset>>,
    mask_interrupts: bool,
}

type BCM2835Registration = hwrng::Registration<BCM2835RNGDev>;
type DeviceData = device::Data<BCM2835Registration, BCM2835Resources, BCM2835General>;

#[vtable]
impl hwrng::Operations for BCM2835RNGDev {
    type Data = Ref<DeviceData>;

    fn read(data: RefBorrow<'_, DeviceData>, buffer: &mut [u8], wait: bool) -> core::result::Result<u32, kernel::Error> {
        let bcm2835 = data.resources().ok_or(ENXIO)?;
        // buffer's length is guaranteed to be a multiple of 4, 
        // thus it's divided by size_of_u32 without a remainder
        let max_words: usize = buffer.len() / size_of::<u32>();
        let mut num_words: usize;
        
        while bcm2835.base.try_readl(RNG_STATUS)? >> 24 == 0 {
            if !wait {
                return Ok(0);
            }

            spin_loop();
        }

        num_words = usize::try_from(bcm2835.base.try_readl(RNG_STATUS)? >> 24)?;
        if num_words > max_words {
            num_words = max_words;
        }

        for i in 0..num_words {
            let word = bcm2835.base.try_readl(RNG_DATA)?;
            for j in 0..4 {
                let byte = (word >> (8 * j)) as u8;
                buffer[i*4 + j] = byte;
            }
        }

        Ok(u32::try_from(num_words * size_of::<u32>())?)
    }

    fn init(data: RefBorrow<'_, DeviceData>) -> Result {
        let mut val: u32;
        let bcm2835 = data.resources().ok_or(ENXIO)?;

        if let Some(rst) = data.reset.lock().take() {
            rst.control_reset()?;
        }

        if data.mask_interrupts {
            // mask the interrupt
            val = bcm2835.base.try_readl(RNG_INT_MASK)?;
            val |= RNG_INT_OFF;
            bcm2835.base.try_writel(val, RNG_INT_MASK)?;
        }

        // set warm-up count & enable
        if bcm2835.base.try_readl(RNG_CTRL)? != RNG_ENABLE {
            bcm2835.base.try_writel(RNG_WARMUP_COUNT, RNG_STATUS)?;
            bcm2835.base.try_writel(RNG_ENABLE, RNG_CTRL)?;
        }

        Ok(())
    }

    fn cleanup(data: Self::Data) {
        // disable rng hardware
        if let Some(bcm2835) = data.resources() {
            bcm2835.base.writel(RNG_DISABLE, RNG_CTRL);
        }

        // disable clock
        if let Some(en_clk) = data.clk.lock().take() {
            en_clk.disable_unprepare();
        }
    }
}

impl Drop for BCM2835RNGDev {
    fn drop(&mut self) {
        pr_info!("BCM2835 RNG Rust driver (exit)\n");
    }
}

struct BCM2835RNGOFData {
    mask_interrupts: bool,
}
const NSP_RNG_OF_DATA: BCM2835RNGOFData = BCM2835RNGOFData { mask_interrupts: true };

struct BCM2835RNGDriver;
impl platform::Driver for BCM2835RNGDriver {
    type Data = Ref<DeviceData>;
    type IdInfo = BCM2835RNGOFData;

    // Called when a new platform device is added or discovered.
    // Implementers should attempt to initialize the device here.
    fn probe(dev: &mut platform::Device, id_info: Option<&Self::IdInfo>) -> Result<Ref<DeviceData>> {
        let mut data = kernel::new_device_data! (
            hwrng::Registration::new(),
            BCM2835Resources {
                base: unsafe { dev.ioremap_resource(0)? },
            },
            BCM2835General {
                _dev: device::Device::from_dev(dev),
                // Before using the clock (and the reset),
                // the driver has to request them at probe execution.
                clk: unsafe { RawSpinLock::new(
                    if let Ok(dev_clk) = dev.clk_get(None) {
                        Some(dev_clk.prepare_enable()?)
                    }
                    else { None }
                )},
                reset: unsafe { RawSpinLock::new(
                    dev.reset_control_get_optional_exclusive(None)?
                )},
                mask_interrupts: {
                    if let Some(idi) = id_info {
                        idi.mask_interrupts
                    }
                    else { false }
                },
            },
            "BCM2835RNG::Registration"
        )?;

        let clk_spinlock = unsafe { 
            data.as_mut().map_unchecked_mut(|d| &mut (**d).clk)
        };
        kernel::rawspinlock_init!(clk_spinlock, "BCM2835RNG::Clock");
        let reset_spinlock = unsafe { 
            data.as_mut().map_unchecked_mut(|d| &mut (**d).reset) 
        };
        kernel::rawspinlock_init!(reset_spinlock, "BCM2835RNG::Reset");

        let data = Ref::<DeviceData>::from(data);

        data.registrations().ok_or(ENXIO)?.as_pinned_mut()
            .register(fmt!("rust_bcm2835_hwrng"), 0, data.clone())?;

        pr_info!("BCM2835 RNG Rust driver registered.\n");

        Ok(data)
    }

    kernel::define_of_id_table! {
        BCM2835RNGOFData,
        [
            (of::DeviceId::Compatible(b"brcm,bcm2835-rng"), None),
            (of::DeviceId::Compatible(b"brcm,bcm-nsp-rng"), Some(NSP_RNG_OF_DATA)),
            (of::DeviceId::Compatible(b"brcm,bcm5301x-rng"), Some(NSP_RNG_OF_DATA)),
            (of::DeviceId::Compatible(b"brcm,bcm6368-rng"), None),
        ]
    }
}

module_platform_driver! {
    type: BCM2835RNGDriver,
    name: "bcm2835_rng_rust",
    author: "Nikos Maragkos <nmaragkos@ceid.upatras.gr>",
    description: "BCM2835 HWRNG Rust driver",
    license: "GPL",
}

