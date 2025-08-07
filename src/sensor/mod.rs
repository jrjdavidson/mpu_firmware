use esp_hal::{i2c::master::I2c, Async};
use mpu6050_dmp::sensor_async::Mpu6050;

pub mod config;
pub mod init;
pub mod motion;
pub type Sensor<'a> = Mpu6050<I2c<'a, Async>>;
