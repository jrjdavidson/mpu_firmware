use crate::shared::{BUZZ_FREQUENCY, MAX_BUZZ_VALUE, MIN_BUZZ_VALUE, PLAY_SOUND};
use defmt::{error, info};
use esp_hal::gpio::AnyPin;
use esp_hal::ledc::{channel, timer, LSGlobalClkSource, Ledc};
use esp_hal_buzzer::Buzzer;

#[embassy_executor::task]
pub async fn buzzer_task(mut ledc: Ledc<'static>, gpio: AnyPin<'static>) {
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

    let mut buzzer = Buzzer::new(
        &ledc,
        timer::Number::Timer0,
        channel::Number::Channel1,
        gpio,
    );
    buzzer.play(1).unwrap_or_else(|e| {
        error!("Failed to initialize buzzer: {}", e);
    });
    buzzer.play(0).unwrap_or_else(|e| {
        error!("Failed to initialize buzzer: {}", e);
    });
    info!("startng buzzer");

    let mut min_value = MIN_BUZZ_VALUE.wait().await;
    let mut max_value = MAX_BUZZ_VALUE.wait().await;
    let mut play_sound = false;
    loop {
        if !play_sound {
            info!("waiting for Sound playback enabled");

            play_sound = PLAY_SOUND.wait().await;
            info!("Sound playback enabled");
        }
        while play_sound {
            // Map to a frequency (e.g., 100 Hz to 2000 Hz)
            let value = BUZZ_FREQUENCY.wait().await;
            min_value = MIN_BUZZ_VALUE.try_take().unwrap_or(min_value);
            max_value = MAX_BUZZ_VALUE.try_take().unwrap_or(max_value);
            info!("vALUE : {}", value);
            let freq = if value > min_value {
                map_to_frequency(value, min_value, max_value)
            } else {
                0
            };
            info!("frequency : {}", freq);

            // Play a tone based on the frequency
            buzzer.play(freq).unwrap_or_else(|e| {
                error!("Failed to play tone: {}", e);
            });
            play_sound = PLAY_SOUND.try_take().unwrap_or(play_sound);
        }
    }
}
fn map_to_frequency(value: u32, min_value: u32, max_value: u32) -> u32 {
    let min_frequency = 100;
    let max_frequency = 2000;
    let range = max_value.saturating_sub(min_value).max(1); // avoid div by zero
    let value = value.clamp(min_value, max_value);
    let freq = (value - min_value) as u64 * (max_frequency - min_frequency) as u64 / range as u64
        + min_frequency as u64;
    freq as u32
}
