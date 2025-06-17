use crate::shared::BLINK_INTERVAL_MS;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

#[embassy_executor::task]
pub async fn led_blink_task(mut led: Output<'static>) {
    loop {
        // Lock and read the current interval
        let interval = *BLINK_INTERVAL_MS.lock().await;

        // Toggle LED
        led.set_high();
        Timer::after(Duration::from_millis(interval as u64)).await;
        led.set_low();
        Timer::after(Duration::from_millis(interval as u64)).await;
    }
}
