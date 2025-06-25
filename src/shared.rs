use defmt::Format;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use heapless::Vec;

#[derive(Debug, Format)]
pub struct SensorData {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
    pub timestamp_ms: u32, // Milliseconds since boot - will overflow after ~49 days
}
impl SensorData {
    pub const fn zero() -> Self {
        Self {
            accel_x: 0,
            accel_y: 0,
            accel_z: 0,
            gyro_x: 0,
            gyro_y: 0,
            gyro_z: 0,
            timestamp_ms: 0,
        }
    }
}
pub trait ToBytes {
    fn write_to_vec(&self, vec: &mut Vec<u8, 16>);
}

impl ToBytes for SensorData {
    fn write_to_vec(&self, vec: &mut Vec<u8, 16>) {
        vec.clear();
        for &val in [
            self.accel_x,
            self.accel_y,
            self.accel_z,
            self.gyro_x,
            self.gyro_y,
            self.gyro_z,
        ]
        .iter()
        {
            vec.extend_from_slice(&val.to_le_bytes()).ok();
        }
        vec.extend_from_slice(&self.timestamp_ms.to_le_bytes()).ok();
    }
}
pub const DEFAULT_READ_INTERVAL_MS: u64 = 5;
pub const DEFAULT_READ_DURATION_S: u16 = 5;

pub static SENSOR_CHANNEL: Channel<CriticalSectionRawMutex, SensorData, 100> = Channel::new();
pub static BLINK_INTERVAL_MS: Signal<CriticalSectionRawMutex, u64> = Signal::new();
pub static READ_INTERVAL_MS: Mutex<CriticalSectionRawMutex, u64> =
    Mutex::new(DEFAULT_READ_INTERVAL_MS);
pub static READ_DURATION_S: Mutex<CriticalSectionRawMutex, u16> =
    Mutex::new(DEFAULT_READ_DURATION_S);
pub static EPOCH: Mutex<CriticalSectionRawMutex, u32> = Mutex::new(0);
pub static BUZZ_FREQUENCY: Signal<CriticalSectionRawMutex, u32> = Signal::new();
pub static MIN_BUZZ_VALUE: Signal<CriticalSectionRawMutex, u32> = Signal::new();
pub static MAX_BUZZ_VALUE: Signal<CriticalSectionRawMutex, u32> = Signal::new();
pub static PLAY_SOUND: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static SOUND_METHOD: Signal<CriticalSectionRawMutex, BuzzFrequencyMode> = Signal::new();

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
