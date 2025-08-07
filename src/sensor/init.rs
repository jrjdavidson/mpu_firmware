use crate::{
    sensor::Sensor,
    shared::{ACCEL_SCALE, GYRO_SCALE},
};
use defmt::info;
use embassy_time::Delay;
use esp_hal::{i2c::master::I2c, Async};
use mpu6050_dmp::{
    accel::AccelFullScale, address::Address, calibration::CalibrationParameters,
    error_async::Error, gyro::GyroFullScale, motion::MotionConfig, sensor_async::Mpu6050,
};

pub async fn initialize_sensor<'a>(
    i2c: I2c<'a, Async>,
) -> Result<Sensor<'a>, Error<I2c<'a, Async>>> {
    let mut sensor = Mpu6050::new(i2c, Address::default()).await.unwrap();

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
    sensor.set_sample_rate_divider(0).await?;
    Ok(sensor)
}

pub async fn configure_sensor<'a>(
    sensor: &mut Mpu6050<I2c<'a, Async>>,
    delay: &mut Delay,
) -> Result<(), Error<I2c<'a, Async>>> {
    // sensor.initialize_dmp(delay).await?;
    let accel_scale = AccelFullScale::G2;
    ACCEL_SCALE.signal(accel_scale as u8);
    let gyro_scale = GyroFullScale::Deg2000;
    GYRO_SCALE.signal(gyro_scale as u8);
    // Configure calibration parameters
    let calibration_params = CalibrationParameters::new(
        accel_scale,
        gyro_scale,
        mpu6050_dmp::calibration::ReferenceGravity::Zero,
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
        threshold: 2, // 0=2mg threshold (minimum possible)
        duration: 10, // 1=1ms at 1kHz sample rate (fastest response)
    };
    sensor.configure_motion_detection(&motion_config).await?;
    sensor.enable_motion_interrupt().await?;
    Ok(())
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
