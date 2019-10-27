#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure PA1 as a tri-state output.
    let mut pin = gpioa.pa1.into_tristate_output();

    // It will be set to floating as the initial state.
    assert_eq!(pin.state().unwrap(), PinState::Floating);

    loop {
        for &state in [PinState::Low, PinState::High, PinState::Floating].iter() {
            pin.set(state).unwrap();
            assert_eq!(pin.state().unwrap(), state);
            delay.delay_ms(500_u16);
        }
    }
}
