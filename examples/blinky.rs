#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::gpio::{
    gpioa, DynamicPin, Floating, Input, OpenDrain, Output, PullUp, PushPull,
};
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

enum MyPin {
    AsOutput(gpioa::PA1<Output<PushPull>>),
    AsInput(gpioa::PA1<Input<Floating>>),
}

impl MyPin {
    fn switcharoo(mut self) {
        self = match self {
            MyPin::AsOutput(p) => MyPin::AsInput(p.into_input_pin().unwrap()),
            MyPin::AsInput(p) => MyPin::AsOutput(p.into_output_pin(PinState::High).unwrap()),
        };
    }
    /*
       fn pull_high(self) {
           match self {
               MyPin::AsOutput(mut p) => {}
               MyPin::AsInput(p) => {p.into_output_pin(PinState::High).unwrap()}
           };
       }

    */
}

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    // Configure PA1 as output.
    let led = gpioa.pa1.into_push_pull_output();

    let pin = MyPin::AsOutput(led);

    let mut iopin = gpioa.pa2.into_dynamic();

    iopin.to_output::<PushPull>(PinState::High).unwrap();

    //    let s = iopin.is_high().unwrap();
    let t = iopin.to_input::<Floating>().unwrap();

    iopin.to_output::<OpenDrain>(PinState::Low).unwrap();

    //    pin.switcharoo();

    //let led_as_input: gpioa::PA1<Input<Floating>> = led.into_input_pin().unwrap();

    //let mut led: gpioa::PA1<Output<PushPull>> = led_as_input.into_output_pin(PinState::High).unwrap();

    loop {}
    /*
        // Set the LED high one million times in a row.
        for _ in 0..1_000_000 {
            led.set_high().unwrap();
        }

        // Set the LED low one million times in a row.
        for _ in 0..1_000_000 {
            led.set_low().unwrap();
        }
    }*/
}
