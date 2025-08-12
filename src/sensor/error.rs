use defmt::Format;
use esp_hal::{i2c::master::I2c, Async};
use mpu6050_dmp::error_async::{Error, InitError};

#[derive(Debug, Format)]
pub enum SensorInitError<'a> {
    Init(InitError<I2c<'a, Async>>),
    Config(Error<I2c<'a, Async>>),
}

impl<'a> From<InitError<I2c<'a, Async>>> for SensorInitError<'a> {
    fn from(err: InitError<I2c<'a, Async>>) -> Self {
        SensorInitError::Init(err)
    }
}

impl<'a> From<Error<I2c<'a, Async>>> for SensorInitError<'a> {
    fn from(err: Error<I2c<'a, Async>>) -> Self {
        SensorInitError::Config(err)
    }
}
