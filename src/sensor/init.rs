use crate::{
    sensor::{config::SensorConfig, error::SensorInitError, Sensor},
    shared::{
        ACCEL_SCALE, BUZZ_FREQUENCY_MODE, DEFAULT_ACCEL_SCALE, DEFAULT_BUZZ_FREQUENCY_MODE,
        DEFAULT_FILTER, DEFAULT_GYRO_SCALE, DEFAULT_MAX_BUZZ_VALUE, DEFAULT_MIN_BUZZ_VALUE,
        DEFAULT_MOTION_DETECTION, DEFAULT_PLAY_SOUND, FILTER, GYRO_SCALE, MAX_BUZZ_VALUE,
        MIN_BUZZ_VALUE, MOTION_DETECTION, PLAY_SOUND,
    },
};
use defmt::info;
use embassy_time::Delay;
use esp_hal::{i2c::master::I2c, Async};
use mpu6050_dmp::{
    address::Address, calibration::CalibrationParameters, motion::MotionConfig,
    sensor_async::Mpu6050,
};

pub async fn initialize_sensor<'a>(i2c: I2c<'a, Async>) -> Result<Sensor<'a>, SensorInitError<'a>> {
    let mut sensor = Mpu6050::new(i2c, Address::default()).await?;

    info!("MPU6050-DMP Sensor Initialized");
    // Configure sensor settings
    // sensor
    //     .set_clock_source(mpu6050_dmp::clock_source::ClockSource::Xgyro)
    //     .await?;

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
    FILTER.signal(DEFAULT_FILTER as u8);
    // Configure DLPF for maximum sensitivity
    sensor.set_digital_lowpass_filter(DEFAULT_FILTER).await?;
    // sensor.initialize_dmp(delay).await?;
    ACCEL_SCALE.signal(DEFAULT_ACCEL_SCALE as u8);
    GYRO_SCALE.signal(DEFAULT_GYRO_SCALE as u8);
    // Configure calibration parameters
    let calibration_params = CalibrationParameters::new(
        DEFAULT_ACCEL_SCALE,
        DEFAULT_GYRO_SCALE,
        mpu6050_dmp::calibration::ReferenceGravity::Zero,
    );

    info!("Calibrating Sensor");
    sensor.calibrate(delay, &calibration_params).await?;

    info!("Sensor Calibrated");
    MOTION_DETECTION.signal(DEFAULT_MOTION_DETECTION);
    let motion_config = MotionConfig {
        threshold: 2,
        duration: 10,
    };
    sensor.configure_motion_detection(&motion_config).await?;
    sensor.enable_motion_interrupt().await?;
    // Configure motion detection with maximum sensitivity
    BUZZ_FREQUENCY_MODE.signal(DEFAULT_BUZZ_FREQUENCY_MODE); //TODO: persist after restart?

    // Set default min/max buzz values
    // These values will be read in the buzzer module, but are initialized here for conistency.
    MIN_BUZZ_VALUE.signal(DEFAULT_MIN_BUZZ_VALUE);
    MAX_BUZZ_VALUE.signal(DEFAULT_MAX_BUZZ_VALUE);
    PLAY_SOUND.signal(DEFAULT_PLAY_SOUND);
    let sensor_config = SensorConfig {
        accel_scale: ACCEL_SCALE.wait().await,
        gyro_scale: GYRO_SCALE.wait().await,
        buzz_frequency_mode: BUZZ_FREQUENCY_MODE.wait().await,
        filter: FILTER.wait().await,
        motion_detection: MOTION_DETECTION.wait().await,
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
