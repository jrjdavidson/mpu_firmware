use crate::shared::BLINK_INTERVAL_MS;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

#[embassy_executor::task]
pub async fn led_blink_task(mut led: Output<'static>) {
    // Wait for the initial blink interval to be set via the signal
    let mut interval = BLINK_INTERVAL_MS.wait().await;
    loop {
        // Turn the LED on
        led.set_high();

        // Create futures for the timer and for a possible interval change
        let mut timer_fut = Timer::after(Duration::from_millis(interval));
        let mut interval_fut = BLINK_INTERVAL_MS.wait();

        // Wait for either the timer to expire or a new interval to be signaled
        match select(&mut timer_fut, &mut interval_fut).await {
            // Timer finished first: turn LED off and repeat the wait for the off period
            Either::First(_) => {
                led.set_low();
                let mut timer_fut = Timer::after(Duration::from_millis(interval));
                let mut interval_fut = BLINK_INTERVAL_MS.wait();
                // Again, wait for either the timer or a new interval
                match select(&mut timer_fut, &mut interval_fut).await {
                    // Timer finished: do nothing, continue to next loop iteration
                    Either::First(_) => {}
                    // New interval received: update and use it for next blink
                    Either::Second(new_interval) => {
                        interval = new_interval;
                    }
                }
            }
            // New interval received before timer finished: update and restart loop
            Either::Second(new_interval) => {
                interval = new_interval;
            }
        }
    }
}
