use defmt::{debug, error, info, warn, Debug2Format};
use embassy_time::{Duration, Instant, Timer, WithTimeout};
use esp_hal::gpio::Input;

use crate::{
    sensor::{
        config::{compute_buzz_frequency, update_sensor_settings, SensorConfig},
        Sensor,
    },
    shared::{
        SensorData, BLINK_INTERVAL_MS, BUZZ_FREQUENCY, CONTINUOUS_SAMPLE_INTERVAL_MS, EPOCH,
        MOTION_READ_DURATION_S, MOTION_SAMPLE_INTERVAL_MS, SENSOR_CHANNEL,
    },
};

#[embassy_executor::task]
pub async fn motion_detection(
    mut sensor: Sensor<'static>,
    mut sensor_config: SensorConfig,
    mut motion_int: Input<'static>,
) {
    // Before entering cyclic measurement, make sure the Interrupt Pin is high
    info!("Starting motion detection");
    // Main loop monitoring motion detection events
    info!("Waiting for motion detection interrupt to be ready");

    loop {
        info!("Waiting for motion detection interrupt");
        let min_interval = *CONTINUOUS_SAMPLE_INTERVAL_MS.lock().await;

        let wait_start = Instant::now();
        let wait_for_high_result = motion_int
            .wait_for_high()
            .with_timeout(Duration::from_millis(min_interval))
            .await; // Wait for motion to stop

        // Wait for hardware interrupt (INT pin going low)
        info!("Motion detection ready");
        // If the wait_for_high timed out, we can continue to the next loop iteration
        let elapsed = Instant::now() - wait_start;
        let remainder = min_interval.saturating_sub(elapsed.as_millis() as u64);
        info!(
            "Motion detected, waiting for INT pin to go low, remaining time: {} ms",
            remainder
        );
        let wait_for_low_result = motion_int
            .wait_for_low()
            .with_timeout(Duration::from_millis(remainder))
            .await;
        if wait_for_low_result.is_err() || wait_for_high_result.is_err() {
            update_sensor_settings(&mut sensor, &mut sensor_config).await;

            // timeout reached, continue to next loop iteration
            sensor = report_motion(sensor, &sensor_config).await;
            continue;
        }
        let mut start = Instant::now();

        let duration = *MOTION_READ_DURATION_S.lock().await as u64;
        // Reset the EPOCH to current time
        *EPOCH.lock().await = start.as_millis() as u32;
        info!("Reading sensor data for {} seconds", duration);
        BLINK_INTERVAL_MS.signal(10);

        update_sensor_settings(&mut sensor, &mut sensor_config).await;

        while Instant::now() - start < Duration::from_secs(duration) {
            // Read current sensor data
            let loop_start = Instant::now();
            sensor = report_motion(sensor, &sensor_config).await;

            // // Monitor motion while it continues
            let motion_check = sensor.check_motion().await;
            match motion_check {
                Ok(result) => {
                    if result.0 {
                        start = Instant::now();
                        info!("Motion detected, resetting start time");
                    }
                }
                Err(e) => {
                    error!("Error when reading motion_check: {}", e);
                }
            }
            //measure how long the loop took so far.
            let elapsed = Instant::now() - loop_start;
            let interval = Duration::from_millis(*MOTION_SAMPLE_INTERVAL_MS.lock().await as u64);
            // If the loop took less time than the interval, wait for the remaining time
            if elapsed < interval {
                Timer::after(interval - elapsed).await;
            }
            if elapsed > interval {
                warn!(
                    "sensor loop interval exceeded Read_interval: {}",
                    elapsed.as_micros()
                );
            }
        }
        info!("No more motion detected");
        BLINK_INTERVAL_MS.signal(1000);
        BUZZ_FREQUENCY.signal(0.0); // Stop buzzer
    }
}

async fn report_motion<'a>(mut sensor: Sensor<'a>, sensor_config: &SensorConfig) -> Sensor<'a> {
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
    sensor
}
