use crate::{
    ble::gatt::Server,
    shared::{ToBytes, SENSOR_CHANNEL},
};
use defmt::{error, info};

use embassy_time::Timer;
use heapless::Vec;
use trouble_host::{gatt::GattConnection, PacketPool};

pub async fn notify_task<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let sensor_accel = &server.imu_service.sensor_accel;
    let sensor_gyro = &server.imu_service.sensor_gyro;
    let mut buf: Vec<u8, 18> = Vec::new();
    let mut accel_batch: Vec<u8, 110> = Vec::new();
    let mut gyro_batch: Vec<u8, 110> = Vec::new();
    loop {
        let mut count = 1;
        accel_batch.clear();
        gyro_batch.clear();
        buf.clear();
        let data = SENSOR_CHANNEL.receive().await;
        data.write_to_vec(&mut buf);
        info!("[custom_task] notifying result");

        //timestamp is at 12..16, accel data at 0..7 (including scale bit at 0), gyro data at 7..14( including scale bit at 7)
        accel_batch.extend_from_slice(&buf[14..18]).ok();
        accel_batch.extend_from_slice(&buf[0..7]).ok();
        gyro_batch.extend_from_slice(&buf[14..18]).ok();
        gyro_batch.extend_from_slice(&buf[7..14]).ok();
        while count < 10 {
            match SENSOR_CHANNEL.try_receive() {
                Ok(data) => {
                    buf.clear();
                    data.write_to_vec(&mut buf);
                    accel_batch.extend_from_slice(&buf[14..18]).ok();
                    accel_batch.extend_from_slice(&buf[0..7]).ok();
                    gyro_batch.extend_from_slice(&buf[14..18]).ok();
                    gyro_batch.extend_from_slice(&buf[7..14]).ok();
                    count += 1;
                }
                Err(_) => break, // Channel empty
            }
        }

        if sensor_accel.notify(conn, &accel_batch).await.is_err() {
            error!("[custom_task] error notifying connection");
            break;
        };
        if sensor_gyro.notify(conn, &gyro_batch).await.is_err() {
            error!("[custom_task] error notifying connection");
            break;
        };
        //throttle notifications, or else will drop connection
        Timer::after_millis(100).await;
    }
}
