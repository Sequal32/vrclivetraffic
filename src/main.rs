mod interpolate;
mod flightradar;
mod flightaware;
mod request;
mod tracker;

use std::{net::TcpListener, io::Write, time::Instant, collections::{HashMap, hash_map::Entry}, fs::File, io::Read};
use serde::Deserialize;

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
    callsign: String
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

    loop {
        println!("Waiting for connection...");
        
        let (mut stream, addr) = listener.accept().unwrap();

        println!("Connection established! {}", addr.to_string());

        stream.write("$DISERVER:CLIENT:LIVE ATC:\r\n".as_bytes()).ok();
        stream.write(format!("$CRSERVER:{0:}:ATC:Y:{0:}\r\n", config.callsign).as_bytes()).ok();

        let mut tracker = Tracker::new(Bounds {
            lat1: config.upper_lat, lat2: config.bottom_lat, lon1: config.upper_lon, lon2: config.bottom_lon
        }, config.floor, config.ceiling);

        let mut injected_tracker: HashMap<String, TrackedData> = HashMap::new();
        let mut timer = Instant::now();
        
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
                let should_interpolate = should_update_position && aircraft.at_last_position_update.unwrap().elapsed().as_secs() < 10;
                if let Err(_) = stream.write(build_aircraft_string(aircraft, should_interpolate).as_bytes()) {break 'main};
                // Give the aircraft a flight plan 
                if !tracked.did_set_fp && aircraft.fp.is_some() {
                    stream.write(build_flightplan_string(&aircraft.data.callsign, aircraft.fp.as_ref().unwrap()).as_bytes()).ok();
                    tracked.did_set_fp = true;
                }
            }

            if should_update_position {
                timer = Instant::now();
                println!("Updating aircraft: {} shown.", aircraft_count);
            }

            // Remove untracked
            for id in injected_tracker.keys().map(|x| x.clone()).collect::<Vec<String>>() {
                if tracker.aircraft_exists(&id) {continue}
                injected_tracker.remove(&id);
            }


            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
