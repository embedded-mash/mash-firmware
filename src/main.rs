#![feature(local_key_cell_methods)]
use anyhow::*;
use embedded_hal::prelude::*;
use embedded_svc::{
    event_bus::EventBus,
    http::server::{registry::Registry, Response},
    timer::{PeriodicTimer, TimerService},
    wifi::*,
};
use esp_idf_hal::{delay::FreeRtos, mutex::Mutex};
use esp_idf_svc::{
    eventloop::{
        EspBackgroundEventLoop, EspEventFetchData, EspEventPostData, EspTypedEventDeserializer,
        EspTypedEventSerializer, EspTypedEventSource,
    },
    http::server::EspHttpServer,
    netif::*,
    nvs::EspDefaultNvs,
    sysloop::EspSysLoopStack,
    timer::{EspTimer, EspTimerService},
    wifi::*,
};
use esp_idf_sys::{c_types, esp_random};
use heapless::String;
use log::*;
use std::{sync::Arc, time::Duration};

const SSID: &str = "FB-WLAN-OG";
#[allow(dead_code)]
const PASS: &str = "wifi";

const INDEX: &str = include_str!("page/index.html");

fn main() -> Result<()> {
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    let default_nvs = Arc::new(EspDefaultNvs::new()?);

    info!("About to start a background event loop");
    let mut event_loop = EspBackgroundEventLoop::new(&Default::default())?;

    println!("wifi started");
    let mut wifi = wifi(netif_stack, sys_loop_stack, default_nvs)?;

    let _subscription = event_loop
        .subscribe(move |message: &EventLoopMessage| {
            match message {
                EventLoopMessage::Command(Command::PropagateNow) => todo!(),
                EventLoopMessage::Command(Command::ScanNetwork) => {
                    if let core::result::Result::Ok(mut ap_infos) = wifi.scan() {
                        ap_infos.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength));
                        let nearest_ssid = ap_infos.pop();

                        let ssid_upper = nearest_ssid.map(|ap| ap.ssid).unwrap_or(SSID.into());

                        println!("{:?}", ssid_upper);
                    }
                }
                EventLoopMessage::Network(NetworkPackage::DropYourConnection(_)) => todo!(),
                EventLoopMessage::Network(NetworkPackage::ReceivesNewPropagation(_)) => todo!(),
                EventLoopMessage::Data(Data(_)) => todo!(),
            };
        })
        .unwrap();

    // event_loop.post(&EventLoopMessage::Water(true), Some(Duration::from_secs(1)))?;
    let arc_event_loop = Arc::new(Mutex::new(event_loop));
    setup_timers(arc_event_loop).unwrap();

    println!("start httpd");
    let _httpd = httpd()?;
    println!("httpd started");

    println!("Starting system loop");
    loop {
        FreeRtos.delay_ms(4u16);
    }
}

fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) -> Result<Box<EspWifi>> {
    let mut wifi = Box::new(EspWifi::new(netif_stack, sys_loop_stack, default_nvs)?);

    // SAFETY: this operation is safe, because we started the EspWiFi module first
    // (https://docs.rs/esp-idf-sys/0.1.2/esp_idf_sys/fn.esp_random.html)
    let ssid_id = unsafe { esp_random() } % 999_999_999;
    let mut ssid: String<32> = String::new();
    ssid.push_str("esp-m-").unwrap();

    for i in 0..9 {
        let char_shift = ((ssid_id / 10u32.pow(i)) % 10u32) as u8;
        ssid.push(('0' as u8 + char_shift) as char).unwrap();
    }

    // SAFETY: As is safe for values between 0 and 13
    let channel = Some(unsafe { esp_random() % 13 } as u8);

    info!("Wifi created, about to scan");
    let mut ap_infos = wifi.scan()?;
    ap_infos.sort_by(|a, b| a.signal_strength.cmp(&b.signal_strength));
    let nearest_ssid = ap_infos.pop();

    let ssid_upper = nearest_ssid.map(|ap| ap.ssid).unwrap_or(SSID.into());

    // wifi.set_configuration(&Configuration::Client(ClientConfiguration {
    //     ssid: SSID.into(),
    //     password: PASS.into(),
    //     channel,
    //     ..Default::default()
    // }))?;

    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: ssid_upper,
            password: PASS.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            // TODO: fix this for the edge-case of a duplication
            ssid,
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    // wifi.set_configuration(&Configuration::AccessPoint(AccessPointConfiguration {
    //     ssid: "ESP-AP".into(),
    //     channel: channel.unwrap_or(1),
    //     ..Default::default()
    // }))?;

    info!("Wifi configuration set, about to get status");

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    match status {
        Status(
            ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(
                _, // ip_settings,
            ))),
            ApStatus::Started(ApIpStatus::Done),
        ) => {
            info!("Wifi connected");
        }
        Status(_, ApStatus::Started(ApIpStatus::Done)) => {
            info!("Soft Ap Started");
        }
        st => {
            bail!("Unexpected Wifi status: {:?}", st);
        }
    }

    Ok(wifi)
}

fn httpd() -> Result<esp_idf_svc::http::server::EspHttpServer> {
    let mut server = EspHttpServer::new(&Default::default())?;

    server.handle_get("/", |_req, resp| {
        resp.send_str(INDEX)?;
        std::result::Result::Ok(())
    })?;
    Ok(server)
}

fn setup_timers(event_loop: Arc<Mutex<EspBackgroundEventLoop>>) -> Result<EspTimer> {
    use embedded_svc::event_bus::Postbox;

    info!("About to schedule a periodic timer every five seconds");
    let mut scan_timer = EspTimerService::new()?.timer(move || {
        info!("Tick from periodic timer");

        event_loop
            .lock()
            .post(&EventLoopMessage::Command(Command::ScanNetwork), None)
            .unwrap();
    })?;
    scan_timer.every(Duration::from_secs(5))?;

    Ok(scan_timer)
}

struct ChildPeer {
    child_peers: PeerState,
}

struct PeerState {
    visible_peers: Vec<u32>,
    child_peers: Vec<ChildPeer>,
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
enum Command {
    PropagateNow,
    ScanNetwork,
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
struct Data(u32);

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
enum NetworkPackage {
    ReceivesNewPropagation(u32),
    DropYourConnection(u32),
}

#[non_exhaustive]
#[derive(Copy, Clone, Debug)]
enum EventLoopMessage {
    Command(Command),
    Network(NetworkPackage),
    Data(Data),
}

impl EspTypedEventSource for EventLoopMessage {
    fn source() -> *const c_types::c_char {
        b"c\0".as_ptr() as *const _
    }
}

impl EspTypedEventSerializer<EventLoopMessage> for EventLoopMessage {
    fn serialize<R>(
        event: &EventLoopMessage,
        f: impl for<'a> FnOnce(&'a EspEventPostData) -> R,
    ) -> R {
        f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), event) })
    }
}

impl EspTypedEventDeserializer<EventLoopMessage> for EventLoopMessage {
    fn deserialize<R>(
        data: &EspEventFetchData,
        f: &mut impl for<'a> FnMut(&'a EventLoopMessage) -> R,
    ) -> R {
        f(unsafe { data.as_payload() })
    }
}
