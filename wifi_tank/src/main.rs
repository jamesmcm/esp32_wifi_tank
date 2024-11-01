#![no_std]
#![no_main]

mod motors;

use core::{mem::MaybeUninit, str::FromStr};
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Config as NetConfig, DhcpConfig, Stack, StackResources};
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{self},
    prelude::*,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_wifi::wifi::{self, AuthMethod, WifiDevice, WifiStaDevice};
use static_cell::StaticCell;

use esp_alloc as _;

type WifiDriver = WifiDevice<'static, WifiStaDevice>;
const CLIENT_NAME: &str = "wifitank";

fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024; // 128KB RAM ought to be enough for anybody
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        esp_alloc::HEAP.add_region(esp_alloc::HeapRegion::new(
            HEAP.as_mut_ptr() as *mut u8,
            HEAP_SIZE,
            esp_alloc::MemoryCapability::Internal.into(),
        ));
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDriver>) -> ! {
    stack.run().await
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let io = esp_hal::gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);

    let seed = 0x0123_4567_89ab_cdef;
    init_heap();
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // Motors
    let left_v_a_pin = gpio::Output::new(io.pins.gpio13, gpio::Level::High);
    let left_v_b_pin = gpio::Output::new(io.pins.gpio12, gpio::Level::High);
    let left_g_a_pin = gpio::Output::new(io.pins.gpio26, gpio::Level::High);
    let left_g_b_pin = gpio::Output::new(io.pins.gpio25, gpio::Level::High);

    let right_v_a_pin = gpio::Output::new(io.pins.gpio21, gpio::Level::High);
    let right_v_b_pin = gpio::Output::new(io.pins.gpio19, gpio::Level::High);
    let right_g_a_pin = gpio::Output::new(io.pins.gpio22, gpio::Level::High);
    let right_g_b_pin = gpio::Output::new(io.pins.gpio23, gpio::Level::High);

    let left_motor =
        motors::MotorDriver::new(left_v_a_pin, left_v_b_pin, left_g_a_pin, left_g_b_pin);
    let right_motor =
        motors::MotorDriver::new(right_v_a_pin, right_v_b_pin, right_g_a_pin, right_g_b_pin);
    let mut motors = motors::Motors::new(left_motor, right_motor);

    esp_println::logger::init_logger_from_env();
    log::info!("Loading");
    let rng = Rng::new(peripherals.RNG);
    let delay = Delay::new();

    let wifi_ssid = heapless::String::<32>::from_str(env!("WIFI_SSID")).unwrap();
    let wifi_password = heapless::String::<64>::from_str(env!("WIFI_PASSWORD")).unwrap();

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    log::info!("Pre-wifi init");
    let wifi_init = esp_wifi::init(
        esp_wifi::EspWifiInitFor::Wifi,
        timg1.timer0,
        rng,
        peripherals.RADIO_CLK,
    )
    .unwrap();

    // TODO: Move this to async task so we can reconnect automatically
    log::info!("Pre-wifi config: {}", &wifi_ssid);
    let wifi_config = esp_wifi::wifi::ClientConfiguration {
        ssid: wifi_ssid,
        bssid: None,
        auth_method: AuthMethod::WPA2Personal, // WPA2Personal - AP is technically WPA3 too, but seems board doesn't support this
        password: wifi_password,
        channel: None,
    };

    log::info!("Post-wifi config");

    log::info!("Pre-wifi creation");
    let (wifi_device, mut wifi_controller): (WifiDevice<WifiStaDevice>, _) =
        wifi::new_with_config(&wifi_init, peripherals.WIFI, wifi_config).unwrap();
    log::info!("Pre-wifi start");
    wifi_controller.start().await.unwrap();
    delay.delay(1000.millis());
    log::info!("Pre-wifi connect");
    for i in 0..20 {
        if wifi_controller.connect().await.is_ok() {
            break;
        }
        log::info!("Failed to connect, retrying - try {}...", i);
        delay.delay(1000.millis());
    }

    // embassy-net setup
    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some(heapless::String::from_str(CLIENT_NAME).unwrap());
    let net_config = NetConfig::dhcpv4(dhcp_config);

    log::info!("Pre-stack assignment");
    static STACK: StaticCell<Stack<WifiDriver>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new(); // Increase this if you start getting socket ring errors.
    let stack = &*STACK.init(Stack::new(
        wifi_device,
        net_config,
        RESOURCES.init(StackResources::<4>::new()),
        seed,
    ));
    let mac_addr = stack.hardware_address();
    log::info!("Hardware configured. MAC Address is {}", mac_addr);

    spawner.spawn(net_task(stack)).unwrap();

    stack.wait_config_up().await;

    match stack.config_v4() {
        Some(a) => log::info!("IP Address appears to be: {}", a.address),
        None => core::panic!("DHCP completed but no IP address was assigned!"),
    }

    let mut udp_rx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_rx_buffer = [0; 1024];
    let mut udp_tx_meta = [PacketMetadata::EMPTY; 16];
    let mut udp_tx_buffer = [0; 1024];
    let mut msg_buffer = [0; 128];

    let mut udp_socket = UdpSocket::new(
        stack,
        &mut udp_rx_meta,
        &mut udp_rx_buffer,
        &mut udp_tx_meta,
        &mut udp_tx_buffer,
    );

    udp_socket.bind(8080).unwrap();

    loop {
        let (rx_size, from_addr) = udp_socket.recv_from(&mut msg_buffer).await.unwrap();
        if rx_size == 0 {
            log::info!("Received empty message from {}", from_addr);
            continue;
        }
        let response = msg_buffer[rx_size - 1] as char;
        match response {
            'F' => motors.forward(),
            'B' => motors.backward(),
            'L' => motors.left(),
            'R' => motors.right(),
            'N' => motors.stop(),
            'Q' => {
                // TODO: Deep sleep here?
                motors.stop();
                break;
            }
            _ => log::info!("Unknown command {}", response),
        }
        // TODO: Add timeout to stop motors if nothing received
    }
}
