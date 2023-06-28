/*!
 * Example of enabling and handling pin change interrupts
 *
 * In this example we can get an interrupt when pin 2 changes
 * and use that to move a stepper motor.
 *
 */
#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::port::{
    mode::{Floating, Input},
    Pin,
};
use avr_device::interrupt::Mutex;
use core::cell::RefCell;
use panic_halt as _;
// use core::sync::atomic::{AtomicBool, Ordering};

enum Turn {
    Left,
    Right,
}

struct RotaryEncoder {
    clock: Pin<Input<Floating>>,
    data: Pin<Input<Floating>>,
    turn: Option<Turn>,
}

impl RotaryEncoder {
    fn new(clock: Pin<Input<Floating>>, data: Pin<Input<Floating>>) -> Self {
        Self {
            clock,
            data,
            turn: None,
        }
    }

    fn notify_turned(&mut self) {
        self.turn = if self.clock.is_high() == self.data.is_high() {
            Some(Turn::Right)
        } else {
            Some(Turn::Left)
        }
    }

    fn check_turn(&mut self) -> Option<Turn> {
        self.turn.take()
    }
}

static ENCODER: Mutex<RefCell<Option<RotaryEncoder>>> = Mutex::new(RefCell::new(None));

//This function is called on change of pin 2
#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn PCINT2() {
    avr_device::interrupt::free(|cs| {
        if let Some(encoder) = ENCODER.borrow(cs).borrow_mut().as_mut() {
            encoder.notify_turned();
        }
    })
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let clock = pins.d2.into_floating_input().downgrade(); //CLK
    let data = pins.d3.into_floating_input().downgrade(); //DT

    avr_device::interrupt::free(|cs| {
        *ENCODER.borrow(cs).borrow_mut() = Some(RotaryEncoder::new(clock, data));
    });

    // Enable the PCINT2 pin change interrupt
    dp.EXINT.pcicr.write(|w| unsafe { w.bits(0b100) });

    // Enable pin change interrupts on PCINT18 which is pin PD2 (= d2)
    dp.EXINT.pcmsk2.write(|w| w.bits(0b100));

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    loop {
        let turn = avr_device::interrupt::free(|cs| {
            if let Some(encoder) = ENCODER.borrow(cs).borrow_mut().as_mut() {
                encoder.check_turn()
            } else {
                None
            }
        });

        if let Some(turn) = turn {
            match turn {
                Turn::Left => ufmt::uwriteln!(serial, "Left").ok(),
                Turn::Right => ufmt::uwriteln!(serial, "Right").ok(),
            };
        }

        // arduino_hal::delay_ms(1000);
        // ufmt::uwriteln!(serial, "Tick").ok();
    }
}
