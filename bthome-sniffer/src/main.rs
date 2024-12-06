
use bluer::{monitor::{Monitor, MonitorEvent, Pattern, RssiSamplingPeriod}, DeviceEvent, DeviceProperty, Uuid};
use bthome::{parse_service_data, BTHOME_UUID, BTHOME_UUID16};
use futures::StreamExt;

const SERVICE_DATA_UUID16: u8 = 0x16;

#[tokio::main(flavor="current_thread")]
async fn main() -> bluer::Result<()> {

    let patterns = vec![
        Pattern { data_type: SERVICE_DATA_UUID16, start_position: 0x00, content: BTHOME_UUID16.to_le_bytes().to_vec() }
    ];

    let session = bluer::Session::new().await?;

    let bthome_uuid = Uuid::from_u128(BTHOME_UUID);

    let adapter = session.default_adapter().await?;

    adapter.set_powered(true).await?;

    let mm = adapter.monitor().await?;
    let mut monitor_handle = mm
        .register(Monitor {
            monitor_type: bluer::monitor::Type::OrPatterns,
            rssi_low_threshold: None,
            rssi_high_threshold: None,
            rssi_low_timeout: None,
            rssi_high_timeout: None,
            rssi_sampling_period: Some(RssiSamplingPeriod::All),
            patterns: Some(patterns),
            ..Default::default()
        })
        .await?;

    while let Some(mevt) = &monitor_handle.next().await {
        let MonitorEvent::DeviceFound(devid) = mevt else {
            continue;
        };
        let dev = adapter.device(devid.device)?;
        let name = dev.name().await?;
        println!("Discovered potential BTHome device {:?} {:?}", devid.device, name);
        if let Ok(Some(service_data)) = dev.service_data().await {
            if let Some(bthome_data) = service_data.get(&bthome_uuid) {
                match parse_service_data(bthome_data.as_slice()) {
                    Ok(bthome_data) => println!("BTHome data is {:?}", bthome_data),
                    Err(err) => println!("Error parsing BTHome data {:?}", err),
                }
            }
        }

        tokio::spawn(async move {
            let mut events = dev.events().await.unwrap();
            while let Some(ev) = events.next().await {
                let DeviceEvent::PropertyChanged(dp) = ev;
                if let DeviceProperty::ServiceData(data) = dp {
                    if let Some(raw_data) = data.get(&bthome_uuid) {
                        println!("Received raw data from bthome device {:0x?}", raw_data);
                        match parse_service_data(raw_data.as_slice()) {
                            Ok(bthome_data) => println!("BTHome data is {:?}", bthome_data),
                            Err(err) => println!("Error parsing BTHome data {:?}", err),
                        }
                    }

                }
            }
        });
    }

    Ok(())
}
