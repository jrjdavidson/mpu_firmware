use defmt::{error, info, warn};
use embassy_futures::join::join;
use embassy_futures::select::select;
use embassy_time::Timer;
use heapless::Vec;
use trouble_host::prelude::*;

use crate::shared::{
    ToBytes, DEFAULT_READ_DURATION_S, DEFAULT_READ_INTERVAL_MS, READ_DURATION_S, READ_INTERVAL_MS,
    SENSOR_CHANNEL,
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
    read_interval: u64,
    #[characteristic(uuid = "12345678-1234-5678-1234-56789abcdef4", write, read, value = DEFAULT_READ_DURATION_S)]
    read_duration: u16,
}

/// Run the BLE stack.
pub async fn run<C>(controller: C)
where
    C: Controller,
{
    // Using a fixed "random" address can be useful for testing. In real scenarios, one would
    // use e.g. the MAC 6 byte array as the address (how to get that varies by the platform).
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
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
    let read_interval = &server.imu_service.read_interval;
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
                    GattEvent::Write(event) => {
                        if event.handle() == read_duration.handle {
                            info!(
                                "[gatt] Write Event to Level Characteristic: {:?}",
                                event.data()
                            );
                            if event.data().len() == 2 {
                                let value = u16::from_le_bytes([event.data()[0], event.data()[1]]);
                                info!("read_duration:{}", value);
                                *READ_DURATION_S.lock().await = value;
                            } else {
                                warn!("[gatt] Write Event to Level Characteristic: invalid data length for u16: {:?}", event.data());
                            }
                        }
                        if event.handle() == read_interval.handle {
                            info!(
                                "[gatt] Write Event to Level Characteristic: {:?}",
                                event.data()
                            );
                            if event.data().len() == 2 {
                                let value = u16::from_le_bytes([event.data()[0], event.data()[1]]);
                                *READ_INTERVAL_MS.lock().await = value as u64;
                            } else {
                                warn!("[gatt] Write Event to Level Characteristic: invalid data length for u16: {:?}", event.data());
                            }
                        }
                    }
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

/// Example task to use the BLE notifier interface.
/// This task will notify the connected central of a counter value every 2 seconds.
/// It will also read the RSSI value every 2 seconds.
/// and will stop when the connection is closed by the central or an error occurs.
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
        Timer::after_millis(20).await;
    }
}
