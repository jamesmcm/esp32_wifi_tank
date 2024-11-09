pub mod espcam;
pub mod wifi_handler;

use std::sync::Arc;

use esp_idf_svc::hal::peripherals::Peripherals;
use espcam::Camera;

use anyhow::{bail, Result};

use esp_idf_hal::io::Write;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::{server::EspHttpServer, Method},
};
use wifi_handler::my_wifi;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let sysloop = EspSystemEventLoop::take()?;

    let peripherals = Peripherals::take().unwrap();
    let wifi_ssid = env!("WIFI_SSID");
    let wifi_password = env!("WIFI_PASSWORD");

    let _wifi = match my_wifi(wifi_ssid, wifi_password, peripherals.modem, sysloop) {
        Ok(inner) => inner,
        Err(err) => {
            bail!("Could not connect to Wi-Fi network: {:?}", err)
        }
    };

    let camera = Camera::new(
        peripherals.pins.gpio32,
        peripherals.pins.gpio0,
        peripherals.pins.gpio5,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        peripherals.pins.gpio21,
        peripherals.pins.gpio36,
        peripherals.pins.gpio39,
        peripherals.pins.gpio34,
        peripherals.pins.gpio35,
        peripherals.pins.gpio25,
        peripherals.pins.gpio23,
        peripherals.pins.gpio22,
        peripherals.pins.gpio26,
        peripherals.pins.gpio27,
        esp_idf_sys::camera::pixformat_t_PIXFORMAT_JPEG,
        // Set quality here
        esp_idf_sys::camera::framesize_t_FRAMESIZE_SVGA,
    )
    .unwrap();
    // camera.sensor().set_vflip(true)?; // Flip if needed

    let cam_arc = Arc::new(camera);
    let cam_arc_clone = cam_arc.clone();

    let mut server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;

    server.fn_handler(
        "/camera",
        Method::Get,
        move |request| -> Result<(), anyhow::Error> {
            let part_boundary = "123456789000000000000987654321";
            let frame_boundary = format!("\r\n--{}\r\n", part_boundary);

            let content_type = format!("multipart/x-mixed-replace;boundary={}", part_boundary);
            let headers = [("Content-Type", content_type.as_str())];
            let mut response = request.into_response(200, Some("OK"), &headers).unwrap();
            loop {
                if let Some(fb) = cam_arc_clone.get_framebuffer() {
                    let data = fb.data();
                    let frame_part = format!(
                        "Content-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                        data.len()
                    );
                    response.write_all(frame_part.as_bytes())?;
                    response.write_all(data)?;
                    response.write_all(frame_boundary.as_bytes())?;
                    response.flush()?;
                }
            }

            Ok(())
        },
    )?;

    server.fn_handler("/", Method::Get, |request| -> Result<(), anyhow::Error> {
        let data = "<html><head><meta name=\"viewport\" content=\"width=device-width; height=device-height;\"><title>esp32cam</title></head><body><img src=\"camera\" alt=\"Failed to load image\" style=\"height: 100%;width: 100%; transform: rotate(180deg);\"></body></html>";


        let headers = [
            ("Content-Type", "text/html"),
            ("Content-Length", &data.len().to_string()),
        ];
        let mut response = request.into_response(200, Some("OK"), &headers)?;
        response.write_all(data.as_bytes())?;
        Ok(())
    })?;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
