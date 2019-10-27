//! General Purpose Input / Output

use core::marker::PhantomData;

use crate::rcc::Rcc;

/// Extension trait to split a GPIO peripheral in independent pins and registers
pub trait GpioExt {
    /// The parts to split the GPIO into
    type Parts;

    /// Splits the GPIO block into independent pins and registers
    fn split(self, rcc: &mut Rcc) -> Self::Parts;
}

/// Input mode (type state)
pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

/// Floating input (type state)
pub struct Floating;

/// Pulled down input (type state)
pub struct PullDown;

/// Pulled up input (type state)
pub struct PullUp;

/// Open drain input or output (type state)
pub struct OpenDrain;

/// Analog mode (type state)
pub struct Analog;

/// Output mode (type state)
pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

/// Push pull output (type state)
pub struct PushPull;

/// Tri-state output (low, high or floating)
pub struct TriState;

/// GPIO Pin speed selection
pub enum Speed {
    Low = 0,
    Medium = 1,
    High = 2,
    VeryHigh = 3,
}

#[allow(dead_code)]
pub(crate) enum AltMode {
    AF0 = 0,
    AF1 = 1,
    AF2 = 2,
    AF3 = 3,
    AF4 = 4,
    AF5 = 5,
    AF6 = 6,
    AF7 = 7,
}

#[cfg(feature = "stm32l0x1")]
#[derive(Copy, Clone)]
pub enum Port {
    PA,
    PB,
}

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
#[derive(Copy, Clone)]
pub enum Port {
    PA,
    PB,
    PC,
    PD,
    PE,
    PH,
}

#[derive(Debug)]
pub enum Error {
    Foo,
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $iopxenr:ident, $PXx:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty),)+
    ]) => {
        /// GPIO
        pub mod $gpiox {
            use core::marker::PhantomData;

            use crate::hal::digital::v2::{
                toggleable, InputPin, OutputPin, StatefulOutputPin, TriStatePin, PinState
            };
            use crate::pac::$GPIOX;
            use crate::rcc::Rcc;
            use super::{
                Floating, GpioExt, Input, OpenDrain, Output, Speed,
                TriState, PullDown, PullUp, PushPull, AltMode, Analog, Port
            };

            /// GPIO parts
            pub struct Parts {
                $(
                    /// Pin
                    pub $pxi: $PXi<$MODE>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self, rcc: &mut Rcc) -> Parts {
                    rcc.rb.iopenr.modify(|_, w| w.$iopxenr().set_bit());

                    Parts {
                        $(
                            $pxi: $PXi {
                                 i: $i,
                                port: Port::$PXx,
                                _mode: PhantomData
                            },
                        )+
                    }
                }
            }

            /// Partially erased pin
            pub struct $PXx<MODE> {
                pub i: u8,
                pub port: Port,
                _mode: PhantomData<MODE>,
            }

            impl<MODE> OutputPin for $PXx<Output<MODE>> {
                type Error = ();

                fn set_high(&mut self) -> Result<(), ()> {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << self.i)) };
                    Ok(())
                }

                fn set_low(&mut self) -> Result<(), ()> {
                    // NOTE(unsafe) atomic write to a stateless register
                    unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << (self.i + 16))) };
                    Ok(())
                }
            }

            impl<MODE> StatefulOutputPin for $PXx<Output<MODE>> {
                fn is_set_high(&self) -> Result<bool, ()> {
                    let is_high = self.is_set_low()?;
                    Ok(is_high)
                }

                fn is_set_low(&self) -> Result<bool, ()> {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_low = unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << self.i) == 0 };
                    Ok(is_low)
                }
            }

            impl<MODE> toggleable::Default for $PXx<Output<MODE>> {}

            impl<MODE> InputPin for $PXx<Output<MODE>> {
                type Error = ();

                fn is_high(&self) -> Result<bool, ()> {
                    let is_high = !self.is_low()?;
                    Ok(is_high)
                }

                fn is_low(&self) -> Result<bool, ()> {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 };
                    Ok(is_low)
                }
            }

            impl<MODE> InputPin for $PXx<Input<MODE>> {
                type Error = ();

                fn is_high(&self) -> Result<bool, ()> {
                    let is_high = !self.is_low()?;
                    Ok(is_high)
                }

                fn is_low(&self) -> Result<bool, ()> {
                    // NOTE(unsafe) atomic read with no side effects
                    let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << self.i) == 0 };
                    Ok(is_low)
                }
            }

            impl TriStatePin for $PXx<TriState> {
                type Error = ();

                fn set(&mut self, state: PinState) -> Result<(), ()> {
                    let offset = 2 * self.i;
                    match state {
                        PinState::Floating => {
                            unsafe {
                                &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                    w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                                });
                            };
                        }
                        PinState::Low | PinState::High => {
                            let sub = if state == PinState::Low { 16 } else { 0 };
                            unsafe {
                                (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << (self.i + sub)));
                                &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                    w.bits(r.bits() & !(0b1 << self.i))
                                });
                                &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                    w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                                });
                            };
                        }
                    }
                    Ok(())
                }

                fn state(&self) -> Result<PinState, ()> {
                    let offset = 2 * self.i;
                    // NOTE(unsafe) atomic read with no side effects
                    let is_input = unsafe {
                        (*$GPIOX::ptr()).moder.read().bits() & (0b11 << offset) == 0
                    };

                    if is_input {
                        Ok(PinState::Floating)
                    } else {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_set_low = unsafe {
                            (*$GPIOX::ptr()).odr.read().bits() & (1 << self.i) == 0
                        };

                        Ok(if is_set_low { PinState::Low } else { PinState::High })
                    }
                }
            }

            $(
                /// Pin
                pub struct $PXi<MODE> {
                    pub i: u8,
                    pub port: Port,
                    _mode: PhantomData<MODE>,
                }

                impl<MODE> $PXi<MODE> {
                    /// Configures the pin to operate as a floating input pin
                    pub fn into_floating_input(
                        self,
                    ) -> $PXi<Input<Floating>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            })
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as a pulled down input pin
                    pub fn into_pull_down_input(
                        self,
                        ) -> $PXi<Input<PullDown>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            })
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as a pulled up input pin
                    pub fn into_pull_up_input(
                        self,
                    ) -> $PXi<Input<PullUp>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            })
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as an analog pin
                    pub fn into_analog(
                        self,
                    ) -> $PXi<Analog> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b11 << offset))
                            });
                        }
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as an open drain output pin
                    pub fn into_open_drain_output(
                        self,
                    ) -> $PXi<Output<OpenDrain>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() | (0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            })
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as an push pull output pin
                    pub fn into_push_pull_output(
                        self,
                    ) -> $PXi<Output<PushPull>> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                w.bits(r.bits() & !(0b1 << $i))
                            });
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                            })
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Configures the pin to operate as a tri-state pin
                    pub fn into_tristate_output(
                        self,
                    ) -> $PXi<TriState> {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                            &(*$GPIOX::ptr()).pupdr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                            });
                        };
                        $PXi {
                             i: $i,
                            port: Port::$PXx,
                            _mode: PhantomData
                        }
                    }

                    /// Set pin speed
                    pub fn set_speed(self, speed: Speed) -> Self {
                        let offset = 2 * $i;
                        unsafe {
                            &(*$GPIOX::ptr()).ospeedr.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | ((speed as u32) << offset))
                            })
                        };
                        self
                    }

                    #[allow(dead_code)]
                    pub(crate) fn set_alt_mode(&self, mode: AltMode) {
                        let mode = mode as u32;
                        let offset = 2 * $i;
                        let offset2 = 4 * $i;
                        unsafe {
                            if offset2 < 32 {
                                &(*$GPIOX::ptr()).afrl.modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            } else {
                                let offset2 = offset2 - 32;
                                &(*$GPIOX::ptr()).afrh.modify(|r, w| {
                                    w.bits((r.bits() & !(0b1111 << offset2)) | (mode << offset2))
                                });
                            }
                            &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                w.bits((r.bits() & !(0b11 << offset)) | (0b10 << offset))
                            });
                        }
                    }
                }

                impl<MODE> $PXi<Output<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Output<MODE>> {
                        $PXx {
                            i: $i,
                            port: Port::$PXx,
                            _mode: self._mode,
                        }
                    }
                }

                impl TriStatePin for $PXi<TriState> {
                    type Error = ();

                    fn set(&mut self, state: PinState) -> Result<(), ()> {
                        let offset = 2 * $i;
                        match state {
                            PinState::Floating => {
                                unsafe {
                                    &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                        w.bits((r.bits() & !(0b11 << offset)) | (0b00 << offset))
                                    });
                                };
                            }
                            PinState::Low | PinState::High => {
                                let sub = if state == PinState::Low { 16 } else { 0 };
                                unsafe {
                                    (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << ($i + sub)));
                                    &(*$GPIOX::ptr()).otyper.modify(|r, w| {
                                        w.bits(r.bits() & !(0b1 << $i))
                                    });
                                    &(*$GPIOX::ptr()).moder.modify(|r, w| {
                                        w.bits((r.bits() & !(0b11 << offset)) | (0b01 << offset))
                                    });
                                };
                            }
                        }
                        Ok(())
                    }

                    fn state(&self) -> Result<PinState, ()> {
                        let offset = 2 * $i;
                        // NOTE(unsafe) atomic read with no side effects
                        let is_input = unsafe {
                            (*$GPIOX::ptr()).moder.read().bits() & (0b11 << offset) == 0
                        };

                        if is_input {
                            Ok(PinState::Floating)
                        } else {
                            // NOTE(unsafe) atomic read with no side effects
                            let is_set_low = unsafe {
                                (*$GPIOX::ptr()).odr.read().bits() & (1 << $i) == 0
                            };

                            Ok(if is_set_low { PinState::Low } else { PinState::High })
                        }
                    }
                }

                impl $PXi<TriState> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<TriState> {
                        $PXx {
                            i: $i,
                            port: Port::$PXx,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> OutputPin for $PXi<Output<MODE>> {
                    type Error = ();

                    fn set_high(&mut self) -> Result<(), ()> {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << $i)) };
                        Ok(())
                    }

                    fn set_low(&mut self) -> Result<(), ()> {
                        // NOTE(unsafe) atomic write to a stateless register
                        unsafe { (*$GPIOX::ptr()).bsrr.write(|w| w.bits(1 << ($i + 16))) };
                        Ok(())
                    }
                }

                impl<MODE> StatefulOutputPin for $PXi<Output<MODE>> {

                    fn is_set_high(&self) -> Result<bool, ()> {
                        let is_set_high = !self.is_set_low()?;
                        Ok(is_set_high)
                    }

                    fn is_set_low(&self) -> Result<bool, ()> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_set_low = unsafe { (*$GPIOX::ptr()).odr.read().bits() & (1 << $i) == 0 };
                        Ok(is_set_low)
                    }
                }

                impl<MODE> toggleable::Default for $PXi<Output<MODE>> {}

                impl<MODE> InputPin for $PXi<Output<MODE>> {
                    type Error = ();

                    fn is_high(&self) -> Result<bool, ()> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&self) -> Result<bool, ()> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }

                impl<MODE> $PXi<Input<MODE>> {
                    /// Erases the pin number from the type
                    ///
                    /// This is useful when you want to collect the pins into an array where you
                    /// need all the elements to have the same type
                    pub fn downgrade(self) -> $PXx<Input<MODE>> {
                        $PXx {
                            i: $i,
                            port: Port::$PXx,
                            _mode: self._mode,
                        }
                    }
                }

                impl<MODE> InputPin for $PXi<Input<MODE>> {

                    type Error = ();

                    fn is_high(&self) -> Result<bool, ()> {
                        let is_high = !self.is_low()?;
                        Ok(is_high)
                    }

                    fn is_low(&self) -> Result<bool, ()> {
                        // NOTE(unsafe) atomic read with no side effects
                        let is_low = unsafe { (*$GPIOX::ptr()).idr.read().bits() & (1 << $i) == 0 };
                        Ok(is_low)
                    }
                }
            )+
        }
    }
}

gpio!(GPIOA, gpioa, iopaen, PA, [
    PA0: (pa0, 0, Input<Floating>),
    PA1: (pa1, 1, Input<Floating>),
    PA2: (pa2, 2, Input<Floating>),
    PA3: (pa3, 3, Input<Floating>),
    PA4: (pa4, 4, Input<Floating>),
    PA5: (pa5, 5, Input<Floating>),
    PA6: (pa6, 6, Input<Floating>),
    PA7: (pa7, 7, Input<Floating>),
    PA8: (pa8, 8, Input<Floating>),
    PA9: (pa9, 9, Input<Floating>),
    PA10: (pa10, 10, Input<Floating>),
    PA11: (pa11, 11, Input<Floating>),
    PA12: (pa12, 12, Input<Floating>),
    PA13: (pa13, 13, Input<Floating>),
    PA14: (pa14, 14, Input<Floating>),
    PA15: (pa15, 15, Input<Floating>),
]);

gpio!(GPIOB, gpiob, iopben, PB, [
    PB0: (pb0, 0, Input<Floating>),
    PB1: (pb1, 1, Input<Floating>),
    PB2: (pb2, 2, Input<Floating>),
    PB3: (pb3, 3, Input<Floating>),
    PB4: (pb4, 4, Input<Floating>),
    PB5: (pb5, 5, Input<Floating>),
    PB6: (pb6, 6, Input<Floating>),
    PB7: (pb7, 7, Input<Floating>),
    PB8: (pb8, 8, Input<Floating>),
    PB9: (pb9, 9, Input<Floating>),
    PB10: (pb10, 10, Input<Floating>),
    PB11: (pb11, 11, Input<Floating>),
    PB12: (pb12, 12, Input<Floating>),
    PB13: (pb13, 13, Input<Floating>),
    PB14: (pb14, 14, Input<Floating>),
    PB15: (pb15, 15, Input<Floating>),
]);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
gpio!(GPIOC, gpioc, iopcen, PC, [
    PC0: (pc0, 0, Input<Floating>),
    PC1: (pc1, 1, Input<Floating>),
    PC2: (pc2, 2, Input<Floating>),
    PC3: (pc3, 3, Input<Floating>),
    PC4: (pc4, 4, Input<Floating>),
    PC5: (pc5, 5, Input<Floating>),
    PC6: (pc6, 6, Input<Floating>),
    PC7: (pc7, 7, Input<Floating>),
    PC8: (pc8, 8, Input<Floating>),
    PC9: (pc9, 9, Input<Floating>),
    PC10: (pc10, 10, Input<Floating>),
    PC11: (pc11, 11, Input<Floating>),
    PC12: (pc12, 12, Input<Floating>),
    PC13: (pc13, 13, Input<Floating>),
    PC14: (pc14, 14, Input<Floating>),
    PC15: (pc15, 15, Input<Floating>),
]);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
gpio!(GPIOD, gpiod, iopcen, PC, [
    PD0: (pd0, 0, Input<Floating>),
    PD1: (pd1, 1, Input<Floating>),
    PD2: (pd2, 2, Input<Floating>),
    PD3: (pd3, 3, Input<Floating>),
    PD4: (pd4, 4, Input<Floating>),
    PD5: (pd5, 5, Input<Floating>),
    PD6: (pd6, 6, Input<Floating>),
    PD7: (pd7, 7, Input<Floating>),
    PD8: (pd8, 8, Input<Floating>),
    PD9: (pd9, 9, Input<Floating>),
    PD10: (pd10, 10, Input<Floating>),
    PD11: (pd11, 11, Input<Floating>),
    PD12: (pd12, 12, Input<Floating>),
    PD13: (pd13, 13, Input<Floating>),
    PD14: (pd14, 14, Input<Floating>),
    PD15: (pd15, 15, Input<Floating>),
]);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
gpio!(GPIOE, gpioe, iopeen, PE, [
    PE0:  (pe0,  0,  Input<Floating>),
    PE1:  (pe1,  1,  Input<Floating>),
    PE2:  (pe2,  2,  Input<Floating>),
    PE3:  (pe3,  3,  Input<Floating>),
    PE4:  (pe4,  4,  Input<Floating>),
    PE5:  (pe5,  5,  Input<Floating>),
    PE6:  (pe6,  6,  Input<Floating>),
    PE7:  (pe7,  7,  Input<Floating>),
    PE8:  (pe8,  8,  Input<Floating>),
    PE9:  (pe9,  9,  Input<Floating>),
    PE10: (pe10, 10, Input<Floating>),
    PE11: (pe11, 11, Input<Floating>),
    PE12: (pe12, 12, Input<Floating>),
    PE13: (pe13, 13, Input<Floating>),
    PE14: (pe14, 14, Input<Floating>),
    PE15: (pe15, 15, Input<Floating>),
]);

#[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
gpio!(GPIOH, gpioh, iophen, PH, [
    PH0: (ph0, 0, Input<Floating>),
    PH1: (ph1, 1, Input<Floating>),
    PH2: (ph2, 2, Input<Floating>),
]);
