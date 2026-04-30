use if_addrs::{get_if_addrs, IfAddr};
use mac_address::get_mac_address;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

fn sanitize_host_label(value: &str) -> String {
    let mut label = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    while label.contains("--") {
        label = label.replace("--", "-");
    }

    label.trim_matches('-').to_string()
}

fn broadcast_from_ip_and_netmask(ip: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Addr {
    let ip = u32::from(ip);
    let netmask = u32::from(netmask);
    Ipv4Addr::from(ip | !netmask)
}

fn collect_network_profile() -> Result<(Vec<IpAddr>, Option<String>), Box<dyn std::error::Error>> {
    let interfaces = get_if_addrs()?
        .into_iter()
        .filter_map(|interface| match interface.addr {
            IfAddr::V4(ipv4) if !ipv4.ip.is_loopback() && !ipv4.ip.is_link_local() => Some((
                IpAddr::V4(ipv4.ip),
                broadcast_from_ip_and_netmask(ipv4.ip, ipv4.netmask),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();

    if interfaces.is_empty() {
        return Err("no non-loopback IPv4 address found for mDNS broadcast".into());
    }

    let ip_addresses = interfaces.iter().map(|(ip, _)| *ip).collect::<Vec<_>>();
    let broadcast_address = interfaces
        .first()
        .map(|(_, broadcast)| broadcast.to_string());

    Ok((ip_addresses, broadcast_address))
}

pub fn local_lan_ip_addresses() -> Vec<IpAddr> {
    collect_network_profile()
        .map(|(ip_addresses, _)| ip_addresses)
        .unwrap_or_default()
}

fn primary_mac_address() -> Option<String> {
    match get_mac_address() {
        Ok(Some(address)) => Some(address.to_string()),
        Ok(None) => None,
        Err(error) => {
            log::warn!(
                "Failed to read local MAC address for Wake-on-LAN: {}",
                error
            );
            None
        }
    }
}

pub fn start_mdns(
    device_id: &str,
    device_name: &str,
    port: u16,
) -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_cobweb._tcp.local.";
    let instance_name = device_name;
    let host_label = sanitize_host_label(device_name);
    let short_device_id = device_id.chars().take(8).collect::<String>();
    let host_name = if host_label.is_empty() {
        format!("cobweb-{}.local.", short_device_id)
    } else {
        format!("{}-{}.local.", host_label, short_device_id)
    };
    let (ip_addresses, broadcast_address) = collect_network_profile()?;
    let mac_address = primary_mac_address();

    let mut properties = HashMap::new();
    properties.insert("deviceId".to_string(), device_id.to_string());
    properties.insert("deviceName".to_string(), device_name.to_string());
    if let Some(mac_address) = mac_address {
        properties.insert("macAddress".to_string(), mac_address);
    }
    if let Some(broadcast_address) = broadcast_address {
        properties.insert("broadcastAddress".to_string(), broadcast_address);
    }

    let service = ServiceInfo::new(
        service_type,
        instance_name,
        &host_name,
        &ip_addresses[..],
        port,
        Some(properties),
    )?
    .enable_addr_auto();

    mdns.register(service)?;
    log::info!(
        "Started mDNS broadcast: {} on port {}, host {}, addresses {:?}",
        service_type,
        port,
        host_name,
        ip_addresses
    );

    Ok(mdns)
}

pub fn stop_mdns(mdns: ServiceDaemon) -> Result<(), Box<dyn std::error::Error>> {
    let receiver = mdns.shutdown()?;
    let _ = receiver.recv();
    Ok(())
}
