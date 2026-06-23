#![no_std]

use embassy_net::Stack;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use esp_hal::gpio::Output;
use log::info;
use picoserve::extract::Query;

pub struct PumpContext {
    output: Mutex<CriticalSectionRawMutex, Output<'static>>,
}

impl PumpContext {
    pub fn new(output: Output<'static>) -> PumpContext {
        PumpContext {
            output: Mutex::new(output),
        }
    }

    pub async fn run_for_duration(&self, duration: Duration) {
        let duration = duration.min(Duration::from_secs(20));

        info!("Pump on ({} seconds).", duration.as_secs());
        self.output.lock().await.set_high();
        embassy_time::Timer::after(duration).await;
        self.output.lock().await.set_low();
        info!("Pump off.");
    }
}

#[derive(serde::Deserialize)]
struct PumpQuery {
    duration: u64,
}

pub async fn start_webserver(pump_context: PumpContext, network_stack: Stack<'static>) -> ! {
    let app = picoserve::Router::new().route(
        "/pump",
        picoserve::routing::get(async |query: Query<PumpQuery>| {
            pump_context
                .run_for_duration(Duration::from_secs(query.duration))
                .await;
            "Ok"
        }),
    );

    info!("Starting web server on port 80");

    let config = picoserve::Config::const_default();
    loop {
        let mut server_rx_buffer = [0; 2048];
        let mut server_tx_buffer = [0; 2048];
        let mut http_buffer = [0; 2048];

        let server = picoserve::Server::new(&app, &config, &mut http_buffer);
        server
            .listen_and_serve(
                0,
                network_stack,
                80,
                &mut server_rx_buffer,
                &mut server_tx_buffer,
            )
            .await;
        embassy_time::Timer::after_secs(5).await;
    }
}
