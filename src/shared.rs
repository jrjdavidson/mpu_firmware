use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use heapless::Vec;

use crate::sensor::config::BuzzFrequencyMode;

#[derive(Debug, Format)]
pub struct SensorData {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub accel_scale: u8,
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
    pub gyro_scale: u8,
    pub timestamp_ms: u32, // Milliseconds since boot - will overflow after ~49 days
}
impl SensorData {
    pub const fn zero() -> Self {
        Self {
            accel_x: 0,
            accel_y: 0,
            accel_z: 0,
            accel_scale: 0,
            gyro_x: 0,
            gyro_y: 0,
            gyro_z: 0,
            gyro_scale: 0,
            timestamp_ms: 0,
        }
    }
}
pub trait ToBytes {
    fn write_to_vec(&self, vec: &mut Vec<u8, 18>);
}

impl ToBytes for SensorData {
    fn write_to_vec(&self, vec: &mut Vec<u8, 18>) {
        vec.clear();

        // accel_scale (u8)
        vec.push(self.accel_scale).ok();

        // accel_x/y/z (i16)
        vec.extend_from_slice(&self.accel_x.to_le_bytes()).ok();
        vec.extend_from_slice(&self.accel_y.to_le_bytes()).ok();
        vec.extend_from_slice(&self.accel_z.to_le_bytes()).ok();

        // gyro_scale (u8)
        vec.push(self.gyro_scale).ok();

        // gyro_x/y/z (i16)
        vec.extend_from_slice(&self.gyro_x.to_le_bytes()).ok();
        vec.extend_from_slice(&self.gyro_y.to_le_bytes()).ok();
        vec.extend_from_slice(&self.gyro_z.to_le_bytes()).ok();

        // timestamp_ms (u32)
        vec.extend_from_slice(&self.timestamp_ms.to_le_bytes()).ok();
    }
}
pub const DEFAULT_MOTION_SAMPLE_INTERVAL_MS: u64 = 10;
pub const DEFAULT_CONTINUOUS_SAMPLE_INTERVAL_MS: u64 = 60000;
pub const DEFAULT_MOTION_READ_DURATION_S: u16 = 5;

pub static SENSOR_CHANNEL: Channel<CriticalSectionRawMutex, SensorData, 100> = Channel::new();
pub static BLINK_INTERVAL_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static MOTION_SAMPLE_INTERVAL_MS: Mutex<CriticalSectionRawMutex, u64> =
    Mutex::new(DEFAULT_MOTION_SAMPLE_INTERVAL_MS);
pub static CONTINUOUS_SAMPLE_INTERVAL_MS: Mutex<CriticalSectionRawMutex, u64> =
    Mutex::new(DEFAULT_CONTINUOUS_SAMPLE_INTERVAL_MS);
pub static MOTION_READ_DURATION_S: Mutex<CriticalSectionRawMutex, u16> =
    Mutex::new(DEFAULT_MOTION_READ_DURATION_S);
pub static EPOCH: Mutex<CriticalSectionRawMutex, u32> = Mutex::new(0);
pub static BUZZ_FREQUENCY: Signal<CriticalSectionRawMutex, f32> = Signal::new();
pub static BUZZ_FREQUENCY_MODE: Signal<CriticalSectionRawMutex, BuzzFrequencyMode> = Signal::new();
pub static MIN_BUZZ_VALUE: Signal<CriticalSectionRawMutex, f32> = Signal::new();
pub static MAX_BUZZ_VALUE: Signal<CriticalSectionRawMutex, f32> = Signal::new();
pub static PLAY_SOUND: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static ACCEL_SCALE: Signal<CriticalSectionRawMutex, u8> = Signal::new();
pub static GYRO_SCALE: Signal<CriticalSectionRawMutex, u8> = Signal::new();
pub static READ: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static MARK_EPOCH: Signal<CriticalSectionRawMutex, ()> = Signal::new();
