use defmt::{error, info, warn};
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_time::Timer;
use heapless::Vec;
use trouble_host::prelude::*;

use crate::shared::{
    ToBytes, DEFAULT_READ_DURATION_S, DEFAULT_READ_INTERVAL_MS, MIN_READ_INTERVAL_MS,
    MOTION_READ_INTERVAL_MS, PLAY_SOUND, READ_DURATION_S, SENSOR_CHANNEL,
};

/// Max number of connections
const CONNECTIONS_MAX: usize = 2;

/// Max number of L2CAP channels.
const L2CAP_CHANNELS_MAX: usize = 2; // Signal + att

// GATT Server definition
#[gatt_server]
struct Server {
    imu_service: MyService,
}
#[gatt_service(uuid = "12345678-1234-5678-1234-56789abcdef0")]
struct MyService {
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef1", read, notify, value =Vec::from_slice(&[0;10]).unwrap())]
    sensor_accel: Vec<u8, 100>,
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef2", read, notify, value =Vec::from_slice(&[0;10]).unwrap())]
    sensor_gyro: Vec<u8, 100>,
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef3", write, read, value = DEFAULT_READ_INTERVAL_MS)]
    motion_read_interval: u64,
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef3", write, read, value = DEFAULT_READ_INTERVAL_MS)]
    min_read_interval: u64,
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef4", write, read, value = DEFAULT_READ_DURATION_S)]
    read_duration: u16,
    #[characteristic(
        uuid = "12345678-1234-5678-1234-56789abcdef5",
        write,
        read,
        value = false
    )]
    play_sound: bool,
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
                        let b = custom_task(&server, &conn);
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

/// Stream Events until the connection closes.
///
/// This function will handle the GATT events and process them.
/// This is how we interact with read and write requests.
async fn gatt_events_task<P: PacketPool>(
    server: &Server<'_>,
    conn: &GattConnection<'_, '_, P>,
) -> Result<(), Error> {
    // let sensor_accel = &server.imu_service.sensor_accel;
    // let sensor_gyro = &server.imu_service.sensor_gyro;
    let read_duration = &server.imu_service.read_duration;
    let motion_read_interval = &server.imu_service.motion_read_interval;
    let min_read_interval = &server.imu_service.min_read_interval;
    let play_sound = &server.imu_service.play_sound;
    let reason = loop {
        match conn.next().await {
            GattConnectionEvent::Disconnected { reason } => break reason,
            GattConnectionEvent::Gatt { event: Err(e) } => {
                warn!("[gatt] error processing event: {:?}", e)
            }
            GattConnectionEvent::Gatt { event: Ok(event) } => {
                match &event {
                    GattEvent::Read(_event) => {
                        // if event.handle() == sensor_imu.handle {
                        //     let value: Result<_, Error> = server.get(sensor_imu);
                        //     info!("[gatt] Read Event to Level Characteristic: {:?}", value);
                        // }
                    }
                    GattEvent::Write(event) => match event.handle() {
                        h if h == read_duration.handle => {
                            handle_u16_write(event.data(), |value| async move {
                                info!("read_duration:{}", value);
                                *READ_DURATION_S.lock().await = value;
                            })
                            .await;
                        }
                        h if h == min_read_interval.handle => {
                            handle_u64_write(event.data(), |value| async move {
                                *MIN_READ_INTERVAL_MS.lock().await = value as u64;
                            })
                            .await;
                        }
                        h if h == motion_read_interval.handle => {
                            handle_u64_write(event.data(), |value| async move {
                                *MOTION_READ_INTERVAL_MS.lock().await = value as u64;
                            })
                            .await;
                        }
                        h if h == play_sound.handle => {
                            let data = event.data();
                            if data.len() == 1 {
                                PLAY_SOUND.signal(data[0] != 0);
                            } else {
                                warn!(
                                    "[gatt] Write Event: invalid data length for u16: {:?}",
                                    data
                                );
                            }
                        }
                        _ => {}
                    },
                };
                // This step is also performed at drop(), but writing it explicitly is necessary
                // in order to ensure reply is sent.
                match event.accept() {
                    Ok(reply) => reply.send().await,
                    Err(e) => warn!("[gatt] error sending response: {:?}", e),
                };
            }
            _ => {} // ignore other Gatt Connection Events
        }
    };
    info!("[gatt] disconnected: {:?}", reason);
    Ok(())
}

// Helper function for u16 GATT writes
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

// Helper function for u64 GATT writes
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

/// Create an advertiser to use to connect to a BLE Central, and wait for it to connect.
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

async fn custom_task<P: PacketPool>(server: &Server<'_>, conn: &GattConnection<'_, '_, P>) {
    let mut tick: u16 = 0;
    let sensor_accel = &server.imu_service.sensor_accel;
    let sensor_gyro = &server.imu_service.sensor_gyro;
    let mut buf: Vec<u8, 16> = Vec::new();
    let mut accel_batch: Vec<u8, 100> = Vec::new();
    let mut gyro_batch: Vec<u8, 100> = Vec::new();
    loop {
        tick = tick.wrapping_add(1);
        let mut count = 0;
        accel_batch.clear();
        gyro_batch.clear();

        info!("[custom_task] notifying connection of tick {}", tick);
        buf.clear();
        let data = SENSOR_CHANNEL.receive().await;
        data.write_to_vec(&mut buf);
        info!("[custom_task] notifying result");

        //timestamp is at 12..16, accel data at 0..6, gyro data at 6..12
        accel_batch.extend_from_slice(&buf[12..16]).ok();
        accel_batch.extend_from_slice(&buf[0..6]).ok();
        gyro_batch.extend_from_slice(&buf[12..16]).ok();
        gyro_batch.extend_from_slice(&buf[6..12]).ok();
        count += 1;
        while count < 10 {
            match SENSOR_CHANNEL.try_receive() {
                Ok(data) => {
                    buf.clear();
                    data.write_to_vec(&mut buf);
                    //timestamp is at 12..16, accel data at 0..6, gyro data at 6..12
                    accel_batch.extend_from_slice(&buf[12..16]).ok();
                    accel_batch.extend_from_slice(&buf[0..6]).ok();
                    gyro_batch.extend_from_slice(&buf[12..16]).ok();
                    gyro_batch.extend_from_slice(&buf[6..12]).ok();
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
