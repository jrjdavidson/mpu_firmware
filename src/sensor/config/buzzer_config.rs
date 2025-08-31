use defmt::Format;
use micromath::F32Ext;
use mpu6050_dmp::{accel::Accel, gyro::Gyro};

use crate::sensor::config::SensorConfig;

#[derive(Clone, Copy, Debug, Format)]
pub enum BuzzFrequencyMode {
    AccelX,
    AccelY,
    AccelZ,
    GyroX,
    GyroY,
    GyroZ,
    AccelMagnitude,
    GyroMagnitude,
}
impl From<u8> for BuzzFrequencyMode {
    fn from(value: u8) -> Self {
        match value {
            0 => BuzzFrequencyMode::AccelX,
            1 => BuzzFrequencyMode::AccelY,
            2 => BuzzFrequencyMode::AccelZ,
            3 => BuzzFrequencyMode::GyroX,
            4 => BuzzFrequencyMode::GyroY,
            5 => BuzzFrequencyMode::GyroZ,
            6 => BuzzFrequencyMode::AccelMagnitude,
            7 => BuzzFrequencyMode::GyroMagnitude,
            _ => BuzzFrequencyMode::AccelX,
        }
    }
}
impl From<BuzzFrequencyMode> for u8 {
    fn from(mode: BuzzFrequencyMode) -> Self {
        match mode {
            BuzzFrequencyMode::AccelX => 0,
            BuzzFrequencyMode::AccelY => 1,
            BuzzFrequencyMode::AccelZ => 2,
            BuzzFrequencyMode::GyroX => 3,
            BuzzFrequencyMode::GyroY => 4,
            BuzzFrequencyMode::GyroZ => 5,
            BuzzFrequencyMode::AccelMagnitude => 6,
            BuzzFrequencyMode::GyroMagnitude => 7,
        }
    }
}
pub fn compute_buzz_frequency(accel: &Accel, gyro: &Gyro, sensor_config: &SensorConfig) -> f32 {
    let mode = sensor_config.buzz_frequency_mode;
    let accel_scale = sensor_config.accel_scale;
    let gyro_scale = sensor_config.gyro_scale;
    match mode.into() {
        BuzzFrequencyMode::AccelX => accel.scaled(accel_scale).x(),
        BuzzFrequencyMode::AccelY => accel.scaled(accel_scale).y(),
        BuzzFrequencyMode::AccelZ => accel.scaled(accel_scale).z(),
        BuzzFrequencyMode::GyroX => gyro.scaled(gyro_scale).x(),
        BuzzFrequencyMode::GyroY => gyro.scaled(gyro_scale).y(),
        BuzzFrequencyMode::GyroZ => gyro.scaled(gyro_scale).z(),
        BuzzFrequencyMode::AccelMagnitude => {
            let accel = accel.scaled(accel_scale);
            let x = accel.x();
            let y = accel.y();
            let z = accel.z();
            (x * x + y * y + z * z).sqrt()
        }
        BuzzFrequencyMode::GyroMagnitude => {
            let gyro = gyro.scaled(gyro_scale);
            let x = gyro.x();
            let y = gyro.y();
            let z = gyro.z();
            (x * x + y * y + z * z).sqrt()
        }
    }
}
