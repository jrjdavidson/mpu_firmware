use crate::shared::{
    BuzzFrequencyMode, SensorData, BLINK_INTERVAL_MS, BUZZ_FREQUENCY, EPOCH, MAX_BUZZ_VALUE,
    MIN_BUZZ_VALUE, READ_DURATION_S, READ_INTERVAL_MS, SENSOR_CHANNEL, SOUND_METHOD,
};
use defmt::{debug, error, info, warn, Debug2Format};
use embassy_time::{Delay, Duration, Instant, Timer};
use esp_hal::{gpio::Input, i2c::master::I2c, Async};
use micromath::F32Ext;
use mpu6050_dmp::{
    accel::Accel, address::Address, calibration::CalibrationParameters, error_async::Error,
    gyro::Gyro, motion::MotionConfig, sensor_async::Mpu6050,
};
pub type Sensor<'a> = Mpu6050<I2c<'a, Async>>;

pub async fn initialize_sensor<'a>(
    i2c: I2c<'a, Async>,
) -> Result<Sensor<'a>, Error<I2c<'a, Async>>> {
    let mut sensor = Mpu6050::new(i2c, Address::default()).await.unwrap();

    info!("MPU6050-DMP Sensor Initialized");
    // Configure sensor settings
    sensor
        .set_clock_source(mpu6050_dmp::clock_source::ClockSource::Xgyro)
        .await?;

    // Set accelerometer full scale to most sensitive range
    sensor
        .set_accel_full_scale(mpu6050_dmp::accel::AccelFullScale::G2)
        .await?;

    // Configure DLPF for maximum sensitivity
    sensor
        .set_digital_lowpass_filter(mpu6050_dmp::config::DigitalLowPassFilter::Filter1)
        .await?;

    // Set sample rate to 1kHz (1ms period)
    sensor.set_sample_rate_divider(0).await?;
    Ok(sensor)
}

pub async fn configure_sensor<'a>(
    sensor: &mut Mpu6050<I2c<'a, Async>>,
    delay: &mut Delay,
) -> Result<(), Error<I2c<'a, Async>>> {
    // sensor.initialize_dmp(delay).await?;

    // Configure calibration parameters
    let calibration_params = CalibrationParameters::new(
        mpu6050_dmp::accel::AccelFullScale::G2,
        mpu6050_dmp::gyro::GyroFullScale::Deg2000,
        mpu6050_dmp::calibration::ReferenceGravity::XN,
    );
    // sensor
    //     .set_accel_calibration(&Accel::new(0, 0, 0))
    //     .await
    //     .?();
    info!("Calibrating Sensor");
    sensor.calibrate(delay, &calibration_params).await?;

    info!("Sensor Calibrated");

    // Configure motion detection with maximum sensitivity
    let motion_config = MotionConfig {
        threshold: 1, // 2mg threshold (minimum possible)
        duration: 1,  // 1ms at 1kHz sample rate (fastest response)
    };
    sensor.configure_motion_detection(&motion_config).await?;
    sensor.enable_motion_interrupt().await?;
    Ok(())
}

#[embassy_executor::task]
pub async fn motion_detection(mut sensor: Sensor<'static>, mut motion_int: Input<'static>) {
    // Before entering cyclic measurement, make sure the Interrupt Pin is high
    info!("Starting motion detection");
    // Main loop monitoring motion detection events
    loop {
        // Wait for hardware interrupt (INT pin going low)
        motion_int.wait_for_high().await;
        info!("Motion detection ready");

        let mut start = Instant::now();
        motion_int.wait_for_low().await;

        // Loop for 10 seconds
        let duration = *READ_DURATION_S.lock().await as u64;
        // Reset the EPOCH to current time
        *EPOCH.lock().await = start.as_millis() as u32;
        info!("Reading sensor data for {} seconds", duration);
        BLINK_INTERVAL_MS.signal(10);
        MIN_BUZZ_VALUE.signal(300);
        MAX_BUZZ_VALUE.signal(10000);
        SOUND_METHOD.signal(BuzzFrequencyMode::AccelX);
        // PLAY_SOUND.signal(true);
        let sound_method = SOUND_METHOD.wait().await;
        while Instant::now() - start < Duration::from_secs(duration) {
            // Read current sensor data
            let loop_start = Instant::now();
            let motion = sensor.motion6().await;
            if let Ok((accel, gyro)) = motion {
                let frequency = compute_buzz_frequency(&accel, &gyro, sound_method);
                BUZZ_FREQUENCY.signal(frequency);
                report_motion(accel, gyro).await;
            }
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
            let interval = Duration::from_millis(*READ_INTERVAL_MS.lock().await as u64);
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
        BUZZ_FREQUENCY.signal(0); // Stop buzzer
    }
}

async fn report_motion<'a>(accel: Accel, gyro: Gyro) {
    let data = SensorData {
        accel_x: accel.x(),
        accel_y: accel.y(),
        accel_z: accel.z(),
        gyro_x: gyro.x(),
        gyro_y: gyro.y(),
        gyro_z: gyro.z(),
        timestamp_ms: embassy_time::Instant::now().as_millis() as u32 - *EPOCH.lock().await,
    };
    if SENSOR_CHANNEL.is_full() {
        //remove oldest data
        debug!("SENSOR_CHANNEL is full, popping oldest data");
        SENSOR_CHANNEL.receive().await;
    }
    debug!("Reporting motion data: {:?}", Debug2Format(&data));
    let send_result = SENSOR_CHANNEL.try_send(data);
    if let Err(send_error) = send_result {
        error!("Send error : {:?}", Debug2Format(&send_error));
    };
}
fn compute_buzz_frequency(accel: &Accel, gyro: &Gyro, mode: BuzzFrequencyMode) -> u32 {
    match mode {
        BuzzFrequencyMode::AccelX => accel.x().abs() as u32,
        BuzzFrequencyMode::AccelY => accel.y().abs() as u32,
        BuzzFrequencyMode::AccelZ => accel.z().abs() as u32,
        BuzzFrequencyMode::GyroX => gyro.x().abs() as u32,
        BuzzFrequencyMode::GyroY => gyro.y().abs() as u32,
        BuzzFrequencyMode::GyroZ => gyro.z().abs() as u32,
        BuzzFrequencyMode::AccelMagnitude => {
            let x = accel.x() as i64;
            let y = accel.y() as i64;
            let z = accel.z() as i64;
            ((x * x + y * y + z * z) as f32).sqrt() as u32
        }
        BuzzFrequencyMode::GyroMagnitude => {
            let x = gyro.x() as i64;
            let y = gyro.y() as i64;
            let z = gyro.z() as i64;
            ((x * x + y * y + z * z) as f32).sqrt() as u32
        }
    }
}
