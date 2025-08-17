use defmt::{debug, error, info, warn, Debug2Format};
use embassy_futures::select::{select, select3, Either, Either3};
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use esp_hal::gpio::Input;

use crate::{
    sensor::{
        config::{compute_buzz_frequency, update_sensor_settings, SensorConfig},
        Sensor,
    },
    shared::{
        SensorData, BLINK_INTERVAL_MS, BUZZ_FREQUENCY, CONTINUOUS_SAMPLE_INTERVAL_MS, EPOCH,
        MARK_EPOCH, MOTION_DETECTION, MOTION_READ_DURATION_S, MOTION_SAMPLE_INTERVAL_MS, READ,
        SENSOR_CHANNEL,
    },
};

#[embassy_executor::task]
pub async fn motion_detection(
    mut sensor: Sensor<'static>,
    mut sensor_config: SensorConfig,
    mut motion_int: Input<'static>,
) {
    info!("Starting motion detection");
    info!("Waiting for motion detection interrupt or READ signal");

    loop {
        let min_interval = *CONTINUOUS_SAMPLE_INTERVAL_MS.lock().await as u64;
        update_sensor_settings(&mut sensor, &mut sensor_config).await;

        info!(
            "Waiting: INT (high->low), READ==true, or {}ms timeout",
            min_interval
        );

        // Build the three competing futures:
        let timer_fut = match min_interval {
            0 => {
                Timer::after(Duration::from_secs(60)) // check settings after 1 sec
            }
            _ => Timer::after(Duration::from_millis(min_interval)),
        };

        // Motion INT: wait for high, then low (edge cycle)
        let motion_fut = async {
            if sensor_config.motion_detection {
                motion_int.wait_for_high().await;
                motion_int.wait_for_low().await;
            } else {
                // see if motion detection signal get updated.
                let motion = MOTION_DETECTION.wait().await;
                // re-signal for sensor_config, so it will get updated on the next loop
                MOTION_DETECTION.signal(motion);
            }
        };

        // READ==true: keep ignoring false signals until we see a true
        let read_true_fut = async {
            loop {
                if READ.wait().await {
                    break;
                }
            }
        };

        // Optional: you can do this here or inside each branch before sampling

        match select3(timer_fut, motion_fut, read_true_fut).await {
            // 1) Periodic timeout: take one sample and loop
            Either3::First(_) => {
                if min_interval != 0 {
                    report_motion(&mut sensor, &sensor_config).await;
                }
                continue;
            }

            // 2) Motion-triggered read window
            Either3::Second(_) => {
                run_read_window(&mut sensor, &mut sensor_config, /*manual*/ false).await;
            }

            // 3) Manual READ-triggered read window
            Either3::Third(_) => {
                run_read_window(&mut sensor, &mut sensor_config, /*manual*/ true).await;
                // Auto-reset READ back to false at the end of the window
                READ.signal(false);
            }
        }
    }
}

async fn run_read_window(sensor: &mut Sensor<'_>, sensor_config: &mut SensorConfig, manual: bool) {
    let duration_s = *MOTION_READ_DURATION_S.lock().await as u64;

    // Reset EPOCH to "now"
    *EPOCH.lock().await = embassy_time::Instant::now().as_millis() as u32;

    info!(
        "Reading sensor data for {} seconds (trigger: {})",
        duration_s,
        if manual { "READ" } else { "INT" }
    );
    BLINK_INTERVAL_MS.signal(10);

    let mut start = Instant::now();
    while Instant::now() - start < Duration::from_secs(duration_s) {
        let loop_start = Instant::now();
        update_sensor_settings(sensor, sensor_config).await; // could settings change wait for next read window?

        // One sample
        report_motion(sensor, &*sensor_config).await;
        let interval = Duration::from_millis(*MOTION_SAMPLE_INTERVAL_MS.lock().await as u64);

        // Extend window if motion continues
        match sensor.check_motion().with_timeout(interval).await {
            Ok(timeout_result) => match timeout_result {
                Ok(check_result) => {
                    if check_result.0 {
                        start = Instant::now();
                        info!("Motion detected, resetting start time");
                    }
                }
                Err(e) => error!("Error when reading motion_check: {}", e),
            },

            Err(e) => error!("Timeout when reading motion_check: {}", e),
        }

        // Keep sample rate, but allow MARK_EPOCH to interrupt the sleep.
        let elapsed = Instant::now() - loop_start;

        if elapsed < interval {
            // Sleep for remainder OR react to MARK_EPOCH immediately
            let remainder = interval - elapsed;

            match select(Timer::after(remainder), MARK_EPOCH.wait()).await {
                Either::First(_) => { /* normal sleep finished */ }
                Either::Second(_) => {
                    let now_ms = Instant::now().as_millis() as u32;
                    *EPOCH.lock().await = now_ms;
                    info!("Epoch marked manually");
                    // Optional: also force a UI blink/buzz change:
                    // BLINK_INTERVAL_MS.signal(10);
                }
            }
        } else {
            warn!(
                "sensor loop interval exceeded Read_interval: {} us",
                elapsed.as_micros()
            );
        }
    }

    info!("No more motion detected");
    BLINK_INTERVAL_MS.signal(1000);
    BUZZ_FREQUENCY.signal(0.0); // Stop buzzer
}

async fn report_motion(sensor: &mut Sensor<'_>, sensor_config: &SensorConfig) {
    let motion = sensor.motion6().await;
    if let Ok((accel, gyro)) = motion {
        let frequency = compute_buzz_frequency(&accel, &gyro, &sensor_config);

        BUZZ_FREQUENCY.signal(frequency);
        let data = SensorData {
            accel_scale: sensor_config.accel_scale,
            accel_x: accel.x(),
            accel_y: accel.y(),
            accel_z: accel.z(),
            gyro_scale: sensor_config.gyro_scale,
            gyro_x: gyro.x(),
            gyro_y: gyro.y(),
            gyro_z: gyro.z(),
            timestamp_ms: embassy_time::Instant::now().as_millis() as u32 - *EPOCH.lock().await,
        };
        if SENSOR_CHANNEL.is_full() {
            //remove oldest data
            warn!("SENSOR_CHANNEL is full, popping oldest data");
            SENSOR_CHANNEL.receive().await;
        }
        debug!("Reporting motion data: {:?}", Debug2Format(&data));
        let send_result = SENSOR_CHANNEL.try_send(data);
        if let Err(send_error) = send_result {
            error!("Send error : {:?}", Debug2Format(&send_error));
        };
    }
}
