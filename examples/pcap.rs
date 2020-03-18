use etherparse::SlicedPacket;
use pcap::Device;
use semtech_udp;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let list = Device::list()?;

    for device in list.iter() {
        if device.name == "any" {
            println!("PCAP Example with flter on port 1680");
            let mut cap = Device::lookup().unwrap().open().unwrap();
            cap.filter("port 1680")?;
            while let Ok(packet) = cap.next() {
                match SlicedPacket::from_ethernet(&packet) {
                    Err(_) => println!("Unparsable packet"),
                    Ok(value) => {
                        let packet = semtech_udp::Packet::parse(
                            &mut value.payload.clone(),
                            value.payload.len(),
                        )?;
                        println!("{:?}", packet);
                    }
                }
            }
        }
    }

    Ok(())
}
