pub mod custom_task;
pub mod events;
pub mod gatt;
use defmt::{error, info, warn};
use embassy_futures::join::join;
use embassy_futures::select::select;
use trouble_host::prelude::*;

/// Max number of connections
const CONNECTIONS_MAX: usize = 2;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

use crate::shared::{
    ACCEL_SCALE, BUZZ_FREQUENCY_MODE, CONTINUOUS_SAMPLE_INTERVAL_MS, GYRO_SCALE,
    MOTION_READ_DURATION_S, MOTION_SAMPLE_INTERVAL_MS, PLAY_SOUND,
};
use custom_task::notify_task;
use gatt::Server;

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

/// Run the BLE stack.
pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random(esp_hal::efuse::Efuse::mac_address());
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        runner,
        ..
    } = stack.build();

    info!("Starting advertising and GATT service");
    if let Ok(server) = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "Motion reporter",
        appearance: &appearance::sensor::GENERIC_SENSOR,
    })) {
        info!(" server created");
        let _ = join(ble_task(runner), async move {
            loop {
                match advertise("Motion reporter", &mut peripheral, &server).await {
                    Ok(conn) => {
                        // set up tasks when the connection is established to a central, so they don't run when no one is connected.
                        info!("[adv] connection established, starting tasks");
                        let a = gatt_events_task(&server, &conn);
                        let b = notify_task(&server, &conn);
                        // run until any task ends (usually because the connection has been closed),
                        // then return to advertising state.
                        select(a, b).await;
                    }
                    Err(e) => {
                        panic!("[adv] error: {:?}", e);
                    }
                }
            }
        })
        .await;
    } else {
        error!("Error starting server");
    };
}

async fn ble_task<C: Controller, P: PacketPool>(mut runner: Runner<'_, C, P>) {
    loop {
        if let Err(e) = runner.run().await {
            panic!("[ble_task] error: {:?}", e);
        }
    }
}
async fn advertise<'values, 'server, C: Controller>(
    name: &'values str,
    peripheral: &mut Peripheral<'values, C, DefaultPacketPool>,
    server: &'server Server<'values>,
) -> Result<GattConnection<'values, 'server, DefaultPacketPool>, BleHostError<C::Error>> {
    let mut advertiser_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ServiceUuids16(&[[0x0f, 0x08]]),
            AdStructure::CompleteLocalName(name.as_bytes()),
        ],
        &mut advertiser_data[..],
    )?;
    let advertiser = peripheral
        .advertise(
            &Default::default(),
            Advertisement::ConnectableScannableUndirected {
                adv_data: &advertiser_data[..len],
                scan_data: &[],
            },
        )
        .await?;
    info!("[adv] advertising");
    let conn = advertiser.accept().await?.with_attribute_server(server)?;
    info!("[adv] connection established");
    Ok(conn)
}
