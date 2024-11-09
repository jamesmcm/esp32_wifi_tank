use core::str;
use std::env;
use std::error::Error;
use tokio::net::UdpSocket;

const TARGET_HOSTNAME: &str = "wifitank:8080";
const CAMERA_HOSTNAME: &str = "espressif:80";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use gilrs::{Button, Event, Gilrs};

    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let mut active_gamepad = None;

    println!("Push button on gamepad!");
    while active_gamepad.is_none() {
        // Examine new events
        while let Some(Event {
            id, event, time, ..
        }) = gilrs.next_event()
        {
            println!("{:?} New event from {}: {:?}", time, id, event);
            active_gamepad = Some(id);
        }
    }

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    println!("Waiting for connection to rover: {}", TARGET_HOSTNAME);

    let socket = UdpSocket::bind(&addr).await?;
    let peer = loop {
        if let Ok(p) = tokio::net::lookup_host(TARGET_HOSTNAME).await {
            if let Some(pi) = p.into_iter().next() {
                break pi;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    };
    println!("Connected to rover:  {}", peer);

    // TODO: Move to other thread?
    println!("Waiting for connection to camera: {}", TARGET_HOSTNAME);
    let camera_peer = loop {
        if let Ok(p) = tokio::net::lookup_host(CAMERA_HOSTNAME).await {
            if let Some(pi) = p.into_iter().next() {
                break pi;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    };
    println!("Connected to camera: {}", camera_peer);
    let cam_url = format!("http://{}", CAMERA_HOSTNAME.split(':').next().unwrap());
    std::process::Command::new("firefox")
        .arg("-kiosk")
        .arg(cam_url.as_str())
        .spawn()
        .ok();

    loop {
        while let Some(Event {
            id, event, time, ..
        }) = gilrs.next_event()
        // TODO: Make this real async?
        {
            println!("{:?} New event from {}: {:?}", time, id, event);
        }

        {
            let gamepad = gilrs.gamepad(active_gamepad.expect("Gamepad not found!"));
            if gamepad.is_pressed(Button::DPadUp) {
                socket.send_to(b"F", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadDown) {
                socket.send_to(b"B", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadLeft) {
                socket.send_to(b"L", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadRight) {
                socket.send_to(b"R", &peer).await?;
            } else if gamepad.is_pressed(Button::Select) {
                socket.send_to(b"Q", &peer).await?;
                break;
            } else {
                socket.send_to(b"N", &peer).await?;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
