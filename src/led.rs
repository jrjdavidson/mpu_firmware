use crate::shared::BLINK_INTERVAL_MS;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

#[embassy_executor::task]
pub async fn led_blink_task(mut led: Output<'static>) {
    let mut interval = BLINK_INTERVAL_MS.wait().await;
    loop {
        // Lock and read the current interval

        // Toggle LED
        led.set_high();
        Timer::after(Duration::from_millis(interval)).await;
        led.set_low();
        Timer::after(Duration::from_millis(interval)).await;

        // Try for a new interval, or continue with the old one
        if let Some(new_interval) = BLINK_INTERVAL_MS.try_take() {
            interval = new_interval;
        }
    }
}
