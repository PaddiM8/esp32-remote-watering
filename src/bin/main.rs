#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use auto_water::{PumpContext, start_webserver};
use embassy_executor::Spawner;
use embassy_net::{Runner, Stack, StackResources};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::peripherals::WIFI;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{Interface, WifiController};
use log::{self, error, info, warn};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32 -o esp32-wroom-32e -o unstable-hal -o alloc -o wifi -o log -o esp-backtrace -o neovim -o vscode -o zed -o esp

    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO2
    // - GPIO5
    // - GPIO12
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO6;
    let _ = peripherals.GPIO7;
    let _ = peripherals.GPIO8;
    let _ = peripherals.GPIO9;
    let _ = peripherals.GPIO10;
    let _ = peripherals.GPIO11;
    let _ = peripherals.GPIO16;
    let _ = peripherals.GPIO20;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    info!("Starting...");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let config = OutputConfig::default();
    let output = Output::new(peripherals.GPIO27, Level::Low, config);

    let network_stack = connect_to_wifi(peripherals.WIFI, &spawner).await;
    let pump_context = PumpContext::new(output);
    start_webserver(pump_context, network_stack.clone()).await

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}

async fn connect_to_wifi(wifi: WIFI<'static>, spawner: &Spawner) -> Stack<'static> {
    info!("Attempting to connect to Wi-Fi...");

    let (mut wifi_controller, interfaces) = esp_radio::wifi::new(wifi, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    let wifi_interface = interfaces.station;
    let station_config = StationConfig::default()
        .with_ssid(env!("WATER_WIFI_SSID"))
        .with_password(env!("WATER_WIFI_PASSWORD").into());
    wifi_controller
        .set_config(&esp_radio::wifi::Config::Station(station_config))
        .unwrap();

    let config = embassy_net::Config::dhcpv4(Default::default());
    static RESOURCES: static_cell::StaticCell<StackResources<3>> = static_cell::StaticCell::new();
    let resources = RESOURCES.init(StackResources::new());
    let rng = Rng::new();
    let random_seed = (rng.random() as u64) << 32 | (rng.random() as u64);
    let (stack, runner) = embassy_net::new(wifi_interface, config, resources, random_seed);
    spawner.spawn(start_runner(runner).unwrap());
    spawner.spawn(start_wifi_connection_loop(wifi_controller).unwrap());

    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}", config.address);
            break;
        }
        embassy_time::Timer::after_millis(500).await;
    }

    stack
}

#[embassy_executor::task]
async fn start_wifi_connection_loop(mut controller: WifiController<'static>) {
    loop {
        if !controller.is_connected() {
            warn!("Wi-Fi not connected. Attempting connection...");
            match controller.connect_async().await {
                Ok(_) => info!("Wi-Fi connected successfully!"),
                Err(e) => error!("Connection failed: {:?}. Retrying...", e),
            }
        }

        embassy_time::Timer::after_secs(5).await;
    }
}

#[embassy_executor::task]
async fn start_runner(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}
