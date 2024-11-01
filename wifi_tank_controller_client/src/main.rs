use core::str;
use std::error::Error;
use std::net::SocketAddr;
use std::{env, io};
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use gilrs::{Button, Event, Gilrs};

    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let mut active_gamepad = None;

    println!("Push button gamepad!");
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

    let socket = UdpSocket::bind(&addr).await?;
    let mut buf = vec![0; 1024];
    println!("Listening on: {}", socket.local_addr()?);

    // Wait to receive message to set peer
    let (receipt_msg_size, peer) = socket.recv_from(&mut buf).await?;
    println!(
        "Received {} bytes from {}: {:?}",
        receipt_msg_size,
        peer,
        str::from_utf8(buf.as_slice()[0..receipt_msg_size].as_ref())
    );
    let mut amt: usize = 0;

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
                amt = socket.send_to(b"F", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadDown) {
                amt = socket.send_to(b"B", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadLeft) {
                amt = socket.send_to(b"L", &peer).await?;
            } else if gamepad.is_pressed(Button::DPadRight) {
                amt = socket.send_to(b"R", &peer).await?;
            } else {
                amt = socket.send_to(b"N", &peer).await?;
            }
        }

        // if let Ok(m) = socket.try_recv_from(&mut buf) {
        //     println!(
        //         "Received {} bytes from {}: {:?}",
        //         m.0,
        //         m.1,
        //         str::from_utf8(buf.as_slice()[0..m.0].as_ref())
        //     );
        // }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
