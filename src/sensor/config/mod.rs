use defmt::info;
use mpu6050_dmp::{accel::AccelFullScale, config::DigitalLowPassFilter, gyro::GyroFullScale};
pub mod buzzer_config;

use crate::{
    sensor::{config::buzzer_config::BuzzFrequencyMode, Sensor},
    shared::{
        ACCEL_SCALE,
        BUZZ_FREQUENCY_MODE,
        DEFAULT_ACCEL_SCALE,
        DEFAULT_BUZZ_FREQUENCY_MODE,
        DEFAULT_FILTER,
        DEFAULT_GYRO_SCALE,
        DEFAULT_MOTION_DETECTION,
        FILTER,
        GYRO_SCALE,
        MOTION_DETECTION, //SENSOR_CHANNEL,
    },
};
pub struct SensorConfig {
    pub accel_scale: AccelFullScale,
    pub gyro_scale: GyroFullScale,
    pub buzz_frequency_mode: BuzzFrequencyMode,
    pub filter: DigitalLowPassFilter,
    pub motion_detection: bool, // use 0 = false, 1 = true
}

impl Into<[u8; 5]> for SensorConfig {
    fn into(self) -> [u8; 5] {
        [
            self.accel_scale as u8,
            self.gyro_scale as u8,
            self.buzz_frequency_mode as u8,
            self.filter as u8,
            self.motion_detection as u8,
        ]
    }
}

impl SensorConfig {
    pub fn apply_buzz_frequency_mode(&mut self, mode_source: Option<BuzzFrequencyMode>) {
        if let Some(new_mode) = mode_source {
            if new_mode as u8 != self.buzz_frequency_mode as u8 {
                info!("Buzz Frequency mode updated: {}", new_mode);
                self.buzz_frequency_mode = new_mode;
            }
        }
    }
    pub async fn apply_accel_scale<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        accel_source: Option<AccelFullScale>,
    ) {
        if let Some(new_accel) = accel_source {
            if new_accel as u8 != self.accel_scale as u8 {
                info!("Accel scale updated: {}", new_accel);

                sensor.set_accel_full_scale(new_accel).await.unwrap();
                self.accel_scale = new_accel;
                //SENSOR_CHANNEL.clear();//not sure if needed?
            }
        }
    }
    pub async fn apply_gyro_scale<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        gyro_source: Option<GyroFullScale>,
    ) {
        if let Some(new_gyro) = gyro_source {
            if new_gyro as u8 != self.gyro_scale as u8 {
                info!("Gyro scale updated: {}", new_gyro);
                sensor.set_gyro_full_scale(new_gyro).await.unwrap();
                self.gyro_scale = new_gyro;
                //SENSOR_CHANNEL.clear();//not sure if needed?
            }
        }
    }
    pub async fn apply_filter<'a>(
        &mut self,
        sensor: &mut Sensor<'a>,
        filter_source: Option<DigitalLowPassFilter>,
    ) {
        if let Some(new_filter) = filter_source {
            if new_filter as u8 != self.filter as u8 {
                info!("Digital Low Pass Filter updated: {}", new_filter);
                sensor.set_digital_lowpass_filter(new_filter).await.unwrap();
                self.filter = new_filter;
            }
        }
    }
    pub fn apply_motion_detection(&mut self, motion_detection: Option<bool>) {
        if let Some(new_detection) = motion_detection {
            if new_detection != self.motion_detection {
                info!("Motion Detection enabled updated: {}", new_detection);
                self.motion_detection = new_detection;
            }
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
// could be rewritten as a single signal of type SENSORCONFIGPACKET, and apply all at once?

pub struct SensorConfigPacket {
    pub accel_scale: u8,
    pub gyro_scale: u8,
    pub buzz_frequency_mode: u8,
    pub filter: u8,
    pub motion_detection: u8, // use 0 = false, 1 = true
}
impl From<[u8; 5]> for SensorConfigPacket {
    fn from(bytes: [u8; 5]) -> Self {
        Self {
            accel_scale: bytes[0],
            gyro_scale: bytes[1],
            buzz_frequency_mode: bytes[2],
            filter: bytes[3],
            motion_detection: bytes[4],
        }
    }
}
impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            accel_scale: DEFAULT_ACCEL_SCALE,
            gyro_scale: DEFAULT_GYRO_SCALE,
            buzz_frequency_mode: DEFAULT_BUZZ_FREQUENCY_MODE,
            filter: DEFAULT_FILTER,
            motion_detection: DEFAULT_MOTION_DETECTION,
        }
    }
}

pub fn signal_new_config(packet: SensorConfigPacket) {
    if let Some(accel_scale) = AccelFullScale::from_u8(packet.accel_scale) {
        ACCEL_SCALE.signal(accel_scale);
    }
    if let Some(gyro_scale) = GyroFullScale::from_u8(packet.gyro_scale) {
        GYRO_SCALE.signal(gyro_scale);
    }
    BUZZ_FREQUENCY_MODE.signal(packet.buzz_frequency_mode.into());
    if let Some(filter) = DigitalLowPassFilter::from_u8(packet.filter) {
        FILTER.signal(filter);
    }
    MOTION_DETECTION.signal(packet.motion_detection != 0);
}
