mod interpolate;
mod flightradar;
mod flightaware;
mod noaa;
mod request;
mod tracker;

use std::{net::TcpListener, io::Write, time::Instant, collections::{HashMap, hash_map::Entry}, fs::File, io::Read};
use serde::Deserialize;
use fsdparser::{Parser, PacketTypes};

use flightaware::FlightPlan;
use flightradar::{Bounds};
use tracker::{TrackData, Tracker};

fn build_aircraft_string(data: &mut TrackData, should_interpolate: bool) -> String {
    let ac_data = &data.data;
    // Calculate PBH
    let h = ac_data.bearing as f64/360.0 * 1024.0;
    let pbh: i32 = 0 << 22 | 0 << 12 | (h as i32) << 2;

    let pos = if should_interpolate {data.position.get()} else {data.position.get_no_update()};

    format!("@N:{callsign}:{squawk}:1:{lat}:{lon}:{alt}:{speed}:{pbh}:0\r\n", 
        callsign = ac_data.callsign,
        squawk = ac_data.squawk_code,
        lat = pos.lat,
        lon = pos.lon,
        alt = ac_data.altitude,
        speed = ac_data.speed,
        pbh = pbh
    )
}

fn build_flightplan_string(callsign: &str, fp: &FlightPlan) -> String {
    format!("$FP{callsign}::I:{equipment}:{speed}:{origin}:0:0:{altitude}:{destination}:0:0:0:0::/v/:{route}\r\n",
        callsign = callsign,
        equipment = fp.equipment,
        speed = fp.speed,
        origin = fp.origin,
        altitude = fp.altitude,
        destination = fp.destination,
        route = fp.route
    )
}

fn build_metar_string(callsign: &str, metar: &String) -> String {
    format!("$ARSERVER:{}:METAR:{}\r\n", callsign, metar)
}

fn build_validate_atc_string(callsign: &str) -> String {
    format!("$CRSERVER:{0:}:ATC:Y:{0:}\r\n", callsign)
}

#[derive(Default)]
struct TrackedData {
    did_set_fp: bool
}

#[derive(Deserialize)]
struct ConfigData {
    upper_lat: f32,
    upper_lon: f32,
    bottom_lat: f32,
    bottom_lon: f32,
    floor: i32,
    ceiling: i32,
}

const CONFIG_FILENAME: &str = "config.json";

fn main() {
    println!("Starting TCP Server");
    let listener = TcpListener::bind("127.0.0.1:6809").unwrap();

    let config: ConfigData;
    {
        // Read from config
        let mut file = File::open(CONFIG_FILENAME).expect("Could not find config.json!");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Could not read config.json!");
        config = serde_json::from_str(String::from_utf8(data).expect("Error decoding file!").as_str()).expect("Config.json is invalid.");
    }
    println!("Read config.json");

    // Weather
    let weather = noaa::NoaaWeather::new();
    weather.run();

    loop {
        println!("Waiting for connection...");
        
        let (mut stream, addr) = listener.accept().unwrap();

        println!("Connection established! {}", addr.to_string());

        // Confirms connection with connect
        stream.write("$DISERVER:CLIENT:LIVE ATC:\r\n".as_bytes()).ok();

        stream.set_nonblocking(true).ok();

        // Instantiate main tracker
        let mut tracker = Tracker::new(Bounds {
            lat1: config.upper_lat, lat2: config.bottom_lat, lon1: config.upper_lon, lon2: config.bottom_lon
        }, config.floor, config.ceiling);
        // Start loops to listen for data
        tracker.run();

        // Map to keep track of data already injected
        let mut injected_tracker: HashMap<String, TrackedData> = HashMap::new();
        let mut timer = Instant::now();
        let mut current_callsign = String::new();
        
        println!("Displaying aircraft...");

        'main: loop {
            tracker.step();

            let should_update_position = timer.elapsed().as_secs_f32() >= 3.0;

            let ac_data = tracker.get_aircraft_data();
            let aircraft_count = ac_data.len();
            for aircraft in ac_data {
                // Insert aircraft as "injected" if not already in
                let tracked: &mut TrackedData = match injected_tracker.entry(aircraft.id.clone()) {
                    Entry::Occupied(o) => o.into_mut(),
                    Entry::Vacant(v) => v.insert(TrackedData::default())
                };
                // Update position either in place or interpolated
                let should_interpolate = !aircraft.data.is_on_ground() && should_update_position && aircraft.at_last_position_update.unwrap().elapsed().as_secs() < 20;
                if let Err(_) = stream.write(build_aircraft_string(aircraft, should_interpolate).as_bytes()) {break 'main};
                // Give the aircraft a flight plan 
                if !tracked.did_set_fp && aircraft.fp.is_some() {
                    stream.write(build_flightplan_string(&aircraft.data.callsign, aircraft.fp.as_ref().unwrap()).as_bytes()).ok();
                    tracked.did_set_fp = true;
                }

                std::thread::sleep(std::time::Duration::from_millis(1));
            }

            // Reset timer
            if should_update_position {
                timer = Instant::now();
                println!("Updating aircraft: {} shown.\r", aircraft_count);
            }

            // Remove untracked
            for id in injected_tracker.keys().map(|x| x.clone()).collect::<Vec<String>>() {
                if tracker.aircraft_exists(&id) {continue}
                injected_tracker.remove(&id);
            }

            // Accept/Parse data from client
            let mut client_request = String::new();
            stream.read_to_string(&mut client_request).ok();
            match Parser::parse(&client_request) {
                Some(packet) => match packet {
                    PacketTypes::Metar(metar) => {
                        if !metar.is_response {
                            weather.request_weather(&metar.payload)
                        }
                    },
                    PacketTypes::ATCPosition(position) => {
                        // Update callsign
                        if current_callsign != position.callsign {
                            // Recognize callsign as a valid controller
                            current_callsign = position.callsign.to_string();
                            stream.write(build_validate_atc_string(&current_callsign).as_bytes()).ok();
                        }
                    }
                    _ => ()
                },
                None => ()
            }

            // Step stuff
            if let Some(Ok(metar)) = weather.get_next_weather() {
                stream.write(build_metar_string(&current_callsign, &metar).as_bytes()).ok();
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
