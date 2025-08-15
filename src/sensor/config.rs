use micromath::F32Ext;
use mpu6050_dmp::{
    accel::{Accel, AccelFullScale},
    gyro::{Gyro, GyroFullScale},
};

use crate::{
    sensor::Sensor,
    shared::{
        ACCEL_SCALE,
        BUZZ_FREQUENCY_MODE,
        GYRO_SCALE,
        MAX_BUZZ_VALUE,
        MIN_BUZZ_VALUE,
        //SENSOR_CHANNEL,
    },
};
pub struct SensorConfig {
    pub accel_scale: u8,
    pub gyro_scale: u8,
    pub buzz_frequency_mode: BuzzFrequencyMode,
    pub min_buzz_value: f32,
    pub max_buzz_value: f32,
}
impl SensorConfig {
    pub fn apply_buzz_frequency_mode(&mut self, mode_source: &mut Option<BuzzFrequencyMode>) {
        if let Some(new_mode) = mode_source.take() {
            self.buzz_frequency_mode = new_mode;
        }
    }
    pub async fn apply_accel_scale<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        accel_source: &mut Option<u8>,
    ) {
        if let Some(new_accel) = accel_source.take() {
            let afs = AccelFullScale::from_u8(new_accel).unwrap_or(AccelFullScale::G2);
            sensor.set_accel_full_scale(afs).await.unwrap();
            self.accel_scale = new_accel;
            //SENSOR_CHANNEL.clear();//not sure if needed?
        }
    }
    pub async fn apply_gyro_scale<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        gyro_source: &mut Option<u8>,
    ) {
        if let Some(new_gyro) = gyro_source.take() {
            let gfs = GyroFullScale::from_u8(new_gyro).unwrap_or(GyroFullScale::Deg2000);
            sensor.set_gyro_full_scale(gfs).await.unwrap();
            self.gyro_scale = new_gyro;
            //SENSOR_CHANNEL.clear();//not sure if needed?
        }
    }
    pub fn apply_max_buzz_value(&mut self, max_source: &mut Option<f32>) {
        if let Some(new_max) = max_source.take() {
            self.max_buzz_value = new_max;
        }
    }
    pub fn apply_min_buzz_value(&mut self, min_source: &mut Option<f32>) {
        if let Some(new_min) = min_source.take() {
            self.min_buzz_value = new_min;
        }
    }
}

pub trait AccelFullScaleFromU8 {
    fn from_u8(value: u8) -> Option<AccelFullScale>;
}

impl AccelFullScaleFromU8 for AccelFullScale {
    fn from_u8(value: u8) -> Option<AccelFullScale> {
        match value {
            0 => Some(AccelFullScale::G2),
            1 => Some(AccelFullScale::G4),
            2 => Some(AccelFullScale::G8),
            3 => Some(AccelFullScale::G16),
            _ => None,
        }
    }
}

pub trait GyroFullScaleFromU8 {
    fn from_u8(value: u8) -> Option<GyroFullScale>;
}

impl GyroFullScaleFromU8 for GyroFullScale {
    fn from_u8(value: u8) -> Option<GyroFullScale> {
        match value {
            0 => Some(GyroFullScale::Deg250),
            1 => Some(GyroFullScale::Deg500),
            2 => Some(GyroFullScale::Deg1000),
            3 => Some(GyroFullScale::Deg2000),
            _ => None,
        }
    }
}
pub async fn update_sensor_settings<'a>(sensor: &mut Sensor<'a>, sensor_config: &mut SensorConfig) {
    sensor_config.apply_buzz_frequency_mode(&mut BUZZ_FREQUENCY_MODE.try_take());
    sensor_config
        .apply_accel_scale(sensor, &mut ACCEL_SCALE.try_take())
        .await;

    sensor_config
        .apply_gyro_scale(sensor, &mut GYRO_SCALE.try_take())
        .await;
    sensor_config.apply_max_buzz_value(&mut MAX_BUZZ_VALUE.try_take());
    sensor_config.apply_min_buzz_value(&mut MIN_BUZZ_VALUE.try_take());
}

#[derive(Clone, Copy, Debug)]
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
    let accel_scale = AccelFullScale::from_u8(sensor_config.accel_scale).unwrap();
    let gyro_scale = GyroFullScale::from_u8(sensor_config.gyro_scale).unwrap();
    match mode {
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
