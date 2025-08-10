use heapless::Vec;
use mpu6050_dmp::{accel::AccelFullScale, gyro::GyroFullScale};
use trouble_host::prelude::*;

use crate::sensor::config::BuzzFrequencyMode;
use crate::shared::{
    DEFAULT_CONTINUOUS_SAMPLE_INTERVAL_MS, DEFAULT_MOTION_READ_DURATION_S,
    DEFAULT_MOTION_SAMPLE_INTERVAL_MS,
};

/// GATT Server definition
#[gatt_server]
pub struct Server {
    pub imu_service: MyService,
}

#[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
pub struct MyService {
    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdeff",
        read,
        value = Vec::from_slice(b"v1.0.0").unwrap()
    )]
    pub firmware_version: Vec<u8, 16>,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef1",
        read,
        notify,
        value = Vec::from_slice(&[0; 11]).unwrap()
    )]
    pub sensor_accel: Vec<u8, 110>,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef2",
        read,
        notify,
        value = Vec::from_slice(&[0; 11]).unwrap()
    )]
    pub sensor_gyro: Vec<u8, 110>,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef3",
        write,
        read,
        value = DEFAULT_CONTINUOUS_SAMPLE_INTERVAL_MS
    )]
    pub continuous_sample_interval: u64,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef4",
        write,
        read,
        value = DEFAULT_MOTION_READ_DURATION_S
    )]
    pub motion_read_duration: u16,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef6",
        write,
        read,
        value = DEFAULT_MOTION_SAMPLE_INTERVAL_MS
    )]
    pub motion_sample_interval: u64,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef7",
        write,
        read,
        value = AccelFullScale::G2 as u8
    )]
    pub accel_scale: u8,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef8",
        write,
        read,
        value = GyroFullScale::Deg2000 as u8
    )]
    pub gyro_scale: u8,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef9",
        write,
        read,
        value = BuzzFrequencyMode::AccelX as u8
    )]
    pub buzz_frequency_mode: u8,

    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef5",
        write,
        read,
        value = false
    )]
    pub play_sound: bool,
}
