use crate::shared::LED_STATE;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LedState {
    Ready,
    Error,
    Calibrating,
    Reading,
    Off,
}
pub enum LedPhase {
    On(Duration),
    Off(Duration),
}

pub struct LedPattern {
    pub phases: &'static [LedPhase],
    pub repeat: bool,
}

pub trait LedSignaler {
    fn signal(&self, state: LedState) -> LedPattern;
}

pub struct DefaultLedSignaler;

const READY_PHASES: &[LedPhase] = &[
    LedPhase::On(Duration::from_millis(1000)),
    LedPhase::Off(Duration::from_millis(1000)),
];
const ERROR_PHASES: &[LedPhase] = &[
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(100)),
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(100)),
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(100)),
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(100)),
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(1100)),
];
const CALIBRATING_PHASES: &[LedPhase] = &[
    LedPhase::On(Duration::from_millis(50)),
    LedPhase::Off(Duration::from_millis(50)),
    LedPhase::On(Duration::from_millis(100)),
    LedPhase::Off(Duration::from_millis(100)),
    LedPhase::On(Duration::from_millis(50)),
    LedPhase::Off(Duration::from_millis(200)),
];
const READING_PHASES: &[LedPhase] = &[
    LedPhase::On(Duration::from_millis(200)),
    LedPhase::Off(Duration::from_millis(200)),
];
const OFF_PHASES: &[LedPhase] = &[];

impl LedSignaler for DefaultLedSignaler {
    fn signal(&self, state: LedState) -> LedPattern {
        match state {
            LedState::Ready => LedPattern {
                phases: READY_PHASES,
                repeat: true,
            },
            LedState::Error => LedPattern {
                phases: ERROR_PHASES,
                repeat: true,
            },
            LedState::Calibrating => LedPattern {
                phases: CALIBRATING_PHASES,
                repeat: true,
            },
            LedState::Reading => LedPattern {
                phases: READING_PHASES,
                repeat: true,
            },
            LedState::Off => LedPattern {
                phases: OFF_PHASES,
                repeat: false,
            },
        }
    }
}
#[embassy_executor::task]
pub async fn led_blink_task(mut led: Output<'static>) {
    let signaler = DefaultLedSignaler;
    let mut current_state = LED_STATE.wait().await;
    let mut pattern = signaler.signal(current_state);

    loop {
        if pattern.phases.is_empty() {
            led.set_low();
            current_state = LED_STATE.wait().await;
            pattern = signaler.signal(current_state);
            continue;
        }

        for phase in pattern.phases {
            let duration = match phase {
                LedPhase::On(d) => {
                    led.set_high();
                    *d
                }
                LedPhase::Off(d) => {
                    led.set_low();
                    *d
                }
            };

            let mut timer = Timer::after(duration);
            let mut state_fut = LED_STATE.wait();

            match select(&mut timer, &mut state_fut).await {
                Either::First(_) => continue,
                Either::Second(new_state) => {
                    current_state = new_state;
                    pattern = signaler.signal(current_state);
                    break;
                }
            }
        }

        if !pattern.repeat {
            // Wait for a new state if the pattern is not repeating
            current_state = LED_STATE.wait().await;
            pattern = signaler.signal(current_state);
        }
    }
}
