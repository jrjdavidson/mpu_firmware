use defmt::{info, warn};
use trouble_host::prelude::*;

use super::gatt::Server;
use crate::shared::{
    ACCEL_SCALE, BUZZ_FREQUENCY_MODE, CONTINUOUS_SAMPLE_INTERVAL_MS, GYRO_SCALE,
    MOTION_READ_DURATION_S, MOTION_SAMPLE_INTERVAL_MS, PLAY_SOUND,
};

/// Stream Events until the connection closes.
///
/// Handles GATT events (especially Writes) and updates shared runtime config/signals.
pub async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    let motion_read_duration = &server.imu_service.motion_read_duration;
    let motion_sample_interval = &server.imu_service.motion_sample_interval;
    let idle_sample_interval = &server.imu_service.continuous_sample_interval;
    let play_sound = &server.imu_service.play_sound;
    let accel_scale = &server.imu_service.accel_scale;
    let gyro_scale = &server.imu_service.gyro_scale;
    let buzz_frequency_mode = &server.imu_service.buzz_frequency_mode;

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
                        h if h == idle_sample_interval.handle => {
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

fn handle_u8_write<F>(data: &[u8], signal_fn: F)
where
    F: Fn(u8),
{
    if data.len() == 1 {
        signal_fn(data[0]);
    } else {
        warn!("[gatt] Write Event: invalid data length for u8: {:?}", data);
    }
}

async fn handle_u16_write<F, Fut>(data: &[u8], mut f: F)
where
    F: FnMut(u16) -> Fut,
    Fut: core::future::Future<Output = ()>,
{
    if data.len() == 2 {
        let value = u16::from_le_bytes([data[0], data[1]]);
        f(value).await;
    } else {
        warn!(
            "[gatt] Write Event: invalid data length for u16: {:?}",
            data
        );
    }
}

async fn handle_u64_write<F, Fut>(data: &[u8], mut f: F)
where
    F: FnMut(u64) -> Fut,
    Fut: core::future::Future<Output = ()>,
{
    if data.len() == 8 {
        let value = u64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ]);
        f(value).await;
    } else {
        warn!(
            "[gatt] Write Event: invalid data length for u64: {:?}",
            data
        );
    }
}
