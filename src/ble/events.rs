use defmt::{info, warn};
use trouble_host::prelude::*;

use super::gatt::Server;
use crate::shared::{
    ACCEL_SCALE, BUZZ_FREQUENCY_MODE, CONTINUOUS_SAMPLE_INTERVAL_MS, FILTER, GYRO_SCALE,
    MARK_EPOCH, MAX_BUZZ_VALUE, MIN_BUZZ_VALUE, MOTION_DETECTION, MOTION_READ_DURATION_S,
    MOTION_SAMPLE_INTERVAL_MS, PLAY_SOUND, READ,
};
use crate::{define_async_write_handler, define_write_handler};
/// Stream Events until the connection closes.
///
/// Handles GATT events (especially Writes) and updates shared runtime config/signals.
pub async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let motion_read_duration = &server.imu_service.motion_read_duration;
    let motion_sample_interval = &server.imu_service.motion_sample_interval;
    let continuous_sample_interval = &server.imu_service.continuous_sample_interval;
    let play_sound = &server.imu_service.play_sound;
    let accel_scale = &server.imu_service.accel_scale;
    let gyro_scale = &server.imu_service.gyro_scale;
    let buzz_frequency_mode = &server.imu_service.buzz_frequency_mode;
    let min_buzz_value = &server.imu_service.min_buzz_value;
    let max_buzz_value = &server.imu_service.max_buzz_value;
    let digital_low_pass_filter = &server.imu_service.digital_low_pass_filter;
    let read = &server.imu_service.read;
    let mark_epoch = &server.imu_service.mark_epoch;
    let motion_detection = &server.imu_service.motion_detection;

    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event } => {
                match &event {
                    GattEvent::Read(_event) => {
                        // Add any ad-hoc read handling here if needed
                    }
                    GattEvent::Write(event) => match event.handle() {
                        h if h == motion_read_duration.handle => {
                            handle_u16_write(event.data(), |value| async move {
                                info!("motion_read_duration: {}", value);
                                *MOTION_READ_DURATION_S.lock().await = value;
                            })
                            .await;
                        }
                        h if h == motion_sample_interval.handle => {
                            handle_u64_write(event.data(), |value| async move {
                                *MOTION_SAMPLE_INTERVAL_MS.lock().await = value as u64;
                            })
                            .await;
                        }
                        h if h == continuous_sample_interval.handle => {
                            handle_u64_write(event.data(), |value| async move {
                                *CONTINUOUS_SAMPLE_INTERVAL_MS.lock().await = value as u64;
                            })
                            .await;
                        }
                        h if h == play_sound.handle => {
                            handle_u8_write(event.data(), |value| PLAY_SOUND.signal(value != 0));
                        }
                        h if h == gyro_scale.handle => {
                            handle_u8_write(event.data(), |value| GYRO_SCALE.signal(value));
                        }
                        h if h == accel_scale.handle => {
                            handle_u8_write(event.data(), |value| ACCEL_SCALE.signal(value));
                        }
                        h if h == buzz_frequency_mode.handle => {
                            handle_u8_write(event.data(), |value| {
                                BUZZ_FREQUENCY_MODE.signal(value.into())
                            });
                        }
                        h if h == min_buzz_value.handle => {
                            handle_f32_write(event.data(), |value| MIN_BUZZ_VALUE.signal(value));
                        }
                        h if h == max_buzz_value.handle => {
                            handle_f32_write(event.data(), |value| MAX_BUZZ_VALUE.signal(value));
                        }
                        h if h == digital_low_pass_filter.handle => {
                            handle_u8_write(event.data(), |value| FILTER.signal(value));
                        }
                        h if h == read.handle => {
                            handle_u8_write(event.data(), |value| READ.signal(value != 0));
                        }
                        h if h == motion_detection.handle => {
                            handle_u8_write(event.data(), |value| {
                                MOTION_DETECTION.signal(value != 0)
                            });
                        }
                        h if h == mark_epoch.handle => {
                            handle_u8_write(event.data(), |value| {
                                if value != 0 {
                                    MARK_EPOCH.signal(())
                                }
                            });
                        }
                        _ => {}
                    },
                    _ => {}
                };

                // Accept + reply: ensure GATT response is sent
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other GATT connection events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

define_write_handler!(handle_u8_write, u8, 1, |d: &[u8]| d[0]);

define_write_handler!(handle_f32_write, f32, 4, |d: &[u8]| f32::from_le_bytes([
    d[0], d[1], d[2], d[3]
]));

define_async_write_handler!(handle_u16_write, u16, 2, |d: &[u8]| u16::from_le_bytes([
    d[0], d[1]
]));

define_async_write_handler!(handle_u64_write, u64, 8, |d: &[u8]| u64::from_le_bytes([
    d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]
]));
