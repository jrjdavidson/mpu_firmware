use crate::{
    sensor::{
        config::{BuzzFrequencyMode, SensorConfig},
        error::SensorInitError,
        Sensor,
    },
    shared::{ACCEL_SCALE, BUZZ_FREQUENCY_MODE, GYRO_SCALE, MAX_BUZZ_VALUE, MIN_BUZZ_VALUE},
};
use defmt::info;
use embassy_time::Delay;
use esp_hal::{i2c::master::I2c, Async};
use mpu6050_dmp::{
    accel::AccelFullScale, address::Address, calibration::CalibrationParameters,
    gyro::GyroFullScale, motion::MotionConfig, sensor_async::Mpu6050,
};

pub async fn initialize_sensor<'a>(i2c: I2c<'a, Async>) -> Result<Sensor<'a>, SensorInitError<'a>> {
    let mut sensor = Mpu6050::new(i2c, Address::default()).await?;

    info!("MPU6050-DMP Sensor Initialized");
    // Configure sensor settings
    // sensor
    //     .set_clock_source(mpu6050_dmp::clock_source::ClockSource::Xgyro)
    //     .await?;

    // // Set accelerometer full scale to most sensitive range
    sensor
        .set_accel_full_scale(mpu6050_dmp::accel::AccelFullScale::G2)
        .await?;

    // Configure DLPF for maximum sensitivity
    sensor
        .set_digital_lowpass_filter(mpu6050_dmp::config::DigitalLowPassFilter::Filter1)
        .await?;

    // Set sample rate to 1kHz (1ms period)
    sensor
        .set_sample_rate_divider(0)
        .await
        .map_err(SensorInitError::Config)?;
    Ok(sensor)
}

pub async fn configure_sensor<'a>(
    sensor: &mut Mpu6050<I2c<'a, Async>>,
    delay: &mut Delay,
) -> Result<SensorConfig, SensorInitError<'a>> {
    // sensor.initialize_dmp(delay).await?;
    let default_accel_scale = AccelFullScale::G2; //TODO: persist after restart?
    ACCEL_SCALE.signal(default_accel_scale as u8);
    let default_gyro_scale = GyroFullScale::Deg2000; //TODO: persist after restart?
    GYRO_SCALE.signal(default_gyro_scale as u8);
    // Configure calibration parameters
    let calibration_params = CalibrationParameters::new(
        default_accel_scale,
        default_gyro_scale,
        mpu6050_dmp::calibration::ReferenceGravity::Zero,
    );

    info!("Calibrating Sensor");
    sensor.calibrate(delay, &calibration_params).await?;

    info!("Sensor Calibrated");
    let motion_detection_enabled = false;
    if !motion_detection_enabled {
        let motion_config = MotionConfig {
            threshold: 2,
            duration: 10,
        };
        sensor.configure_motion_detection(&motion_config).await?;
        sensor.enable_motion_interrupt().await?;
    }
    // Configure motion detection with maximum sensitivity
    let default_buzz_frequency_mode = BuzzFrequencyMode::AccelX;
    BUZZ_FREQUENCY_MODE.signal(default_buzz_frequency_mode); //TODO: persist after restart?
    let default_min_buzz_value = 0.5; //TODO: persist after restart?
    MIN_BUZZ_VALUE.signal(default_min_buzz_value);
    let default_max_buzz_value = 2.0; //TODO: persist after restart?
    MAX_BUZZ_VALUE.signal(default_max_buzz_value);
    let sensor_config = SensorConfig {
        accel_scale: ACCEL_SCALE.wait().await,
        gyro_scale: GYRO_SCALE.wait().await,
        buzz_frequency_mode: BUZZ_FREQUENCY_MODE.wait().await,
        min_buzz_value: default_min_buzz_value, // is "waited" in the buzzer thread
        max_buzz_value: default_max_buzz_value, // is "waited" in the buzzer thread
    };
    Ok(sensor_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpu6050_dmp::accel::Accel;
    use mpu6050_dmp::gyro::Gyro;

    #[test]
    fn test_compute_buzz_frequency_handles_i16_min() {
        // Accel and Gyro with x = i16::MIN, others arbitrary
        let accel = Accel::new(i16::MIN, -100, 200);
        let gyro = Gyro::new(i16::MIN, 0, 0);

        // Should not panic, should return 32768 for unsigned_abs
        assert_eq!(
            compute_buzz_frequency(&accel, &gyro, BuzzFrequencyMode::AccelX),
            32768
        );
        assert_eq!(
            compute_buzz_frequency(&gyro, &gyro, BuzzFrequencyMode::GyroX),
            32768
        );

        // Magnitude should also not panic and return a valid value
        let mag = compute_buzz_frequency(&accel, &gyro, BuzzFrequencyMode::AccelMagnitude);
        assert!(mag > 32768);

        let mag_gyro = compute_buzz_frequency(&accel, &gyro, BuzzFrequencyMode::GyroMagnitude);
        assert!(mag_gyro >= 32768);
    }
}
