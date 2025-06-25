#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
use bt_hci::controller::ExternalController;
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Delay;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::ledc::Ledc;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use mputest::led::led_blink_task;
use mputest::sensor::{configure_sensor, initialize_sensor, motion_detection};
use mputest::shared::BLINK_INTERVAL_MS;
use mputest::{ble, buzzer};
use panic_rtt_target as _;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.4.0

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let led = Output::new(peripherals.GPIO15, Level::High, OutputConfig::default());
    spawner.spawn(led_blink_task(led)).ok();

    let ledc = Ledc::new(peripherals.LEDC);
    let buzzer_gpio = peripherals.GPIO19;

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK)
        .expect("Failed to initialize WIFI/BLE controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(&wifi_init, peripherals.BT);
    let ble_controller = ExternalController::<_, 20>::new(transport);
    // Initialize I2C and sensor
    let sda = peripherals.GPIO1;
    let scl = peripherals.GPIO0;

    // Configure GPIO16 as interrupt input with pull-up
    let motion_int: Input<'_> = Input::new(
        peripherals.GPIO2,
        InputConfig::default().with_pull(Pull::Up),
    );

    let config = esp_hal::i2c::master::Config::default();
    let bus = esp_hal::i2c::master::I2c::new(peripherals.I2C0, config)
        .unwrap()
        .with_scl(scl)
        .with_sda(sda)
        .into_async();
    let sensor_result = initialize_sensor(bus).await;
    let mut sensor = match sensor_result {
        Ok(sensor) => sensor,
        Err(e) => {
            info!("Failed to initialize sensor: {:?}", e);
            BLINK_INTERVAL_MS.signal(100);
            return;
        }
    };
    let mut delay = Delay;
    BLINK_INTERVAL_MS.signal(200);
    let sensor_config = configure_sensor(&mut sensor, &mut delay).await;
    match sensor_config {
        Ok(_) => info!("Sensor configured successfully"),
        Err(e) => {
            info!("Failed to configure sensor: {:?}", e);
            BLINK_INTERVAL_MS.signal(100);

            return;
        }
    }
    BLINK_INTERVAL_MS.signal(1000);

    spawner
        .spawn(buzzer::buzzer_task(ledc, buzzer_gpio.into()))
        .ok();

    spawner.spawn(motion_detection(sensor, motion_int)).ok();
    ble::run(ble_controller).await;
    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.1/examples/src/bin
}
