use defmt::{info, Format};
use micromath::F32Ext;
use mpu6050_dmp::{
    accel::{Accel, AccelFullScale},
    config::DigitalLowPassFilter,
    gyro::{Gyro, GyroFullScale},
};

use crate::{
    sensor::Sensor,
    shared::{
        ACCEL_SCALE,
        BUZZ_FREQUENCY_MODE,
        FILTER,
        GYRO_SCALE,
        MOTION_DETECTION, //SENSOR_CHANNEL,
    },
};
pub struct SensorConfig {
    pub accel_scale: u8,
    pub gyro_scale: u8,
    pub buzz_frequency_mode: BuzzFrequencyMode,
    pub filter: u8,
    pub motion_detection: bool,
}
impl SensorConfig {
    pub fn apply_buzz_frequency_mode(&mut self, mode_source: Option<BuzzFrequencyMode>) {
        if let Some(new_mode) = mode_source {
            info!("Buzz Frequency mode updated: {}", new_mode);
            self.buzz_frequency_mode = new_mode;
        }
    }
    pub async fn apply_accel_scale<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        accel_source: Option<u8>,
    ) {
        if let Some(new_accel) = accel_source {
            let afs = AccelFullScale::from_u8(new_accel).unwrap_or(AccelFullScale::G2);
            info!("Accel scale updated: {}", afs);

            sensor.set_accel_full_scale(afs).await.unwrap();
            self.accel_scale = new_accel;
            //SENSOR_CHANNEL.clear();//not sure if needed?
        }
    }
    pub async fn apply_gyro_scale<'a>(&mut self, sensor: &mut Sensor<'a>, gyro_source: Option<u8>) {
        if let Some(new_gyro) = gyro_source {
            let gfs = GyroFullScale::from_u8(new_gyro).unwrap_or(GyroFullScale::Deg2000);
            info!("Gyro scale updated: {}", gfs);
            sensor.set_gyro_full_scale(gfs).await.unwrap();
            self.gyro_scale = new_gyro;
            //SENSOR_CHANNEL.clear();//not sure if needed?
        }
    }
    pub async fn apply_filter<'a>(&mut self, sensor: &mut Sensor<'a>, filter_source: Option<u8>) {
        if let Some(new_filter) = filter_source {
            let dlpf =
                DigitalLowPassFilter::from_u8(new_filter).unwrap_or(DigitalLowPassFilter::Filter1);
            info!("Digital Low Pass Filter updated: {}", dlpf);
            sensor.set_digital_lowpass_filter(dlpf).await.unwrap();
            self.filter = new_filter;
            //SENSOR_CHANNEL.clear();//not sure if needed?
        }
    }
    pub fn apply_motion_detection(&mut self, motion_detection: Option<bool>) {
        if let Some(new_detection) = motion_detection {
            info!("Motion Detection enabled updated: {}", new_detection);

            self.motion_detection = new_detection;
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

pub trait DigitalLowPassFilterFromU8 {
    fn from_u8(value: u8) -> Option<DigitalLowPassFilter>;
}

impl DigitalLowPassFilterFromU8 for DigitalLowPassFilter {
    fn from_u8(value: u8) -> Option<DigitalLowPassFilter> {
        match value {
            0 => Some(DigitalLowPassFilter::Filter0),
            1 => Some(DigitalLowPassFilter::Filter1),
            2 => Some(DigitalLowPassFilter::Filter2),
            3 => Some(DigitalLowPassFilter::Filter3),
            4 => Some(DigitalLowPassFilter::Filter4),
            5 => Some(DigitalLowPassFilter::Filter5),
            6 => Some(DigitalLowPassFilter::Filter6),
            _ => None,
        }
    }
}
pub async fn update_sensor_settings<'a>(sensor: &mut Sensor<'a>, sensor_config: &mut SensorConfig) {
    sensor_config.apply_buzz_frequency_mode(BUZZ_FREQUENCY_MODE.try_take());
    sensor_config
        .apply_accel_scale(sensor, ACCEL_SCALE.try_take())
        .await;

    sensor_config
        .apply_gyro_scale(sensor, GYRO_SCALE.try_take())
        .await;
    sensor_config.apply_filter(sensor, FILTER.try_take()).await;

    sensor_config.apply_motion_detection(MOTION_DETECTION.try_take());
}

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
